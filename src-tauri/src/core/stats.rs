use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use std::collections::HashMap;

use crate::core::model::{FocusSession, Routine, StreakRule};

pub fn day_of(dt: DateTime<Utc>, tz: FixedOffset) -> NaiveDate {
    dt.with_timezone(&tz).date_naive()
}

pub fn seconds_per_routine(
    sessions: &[FocusSession],
    day: NaiveDate,
    tz: FixedOffset,
) -> HashMap<i64, i64> {
    let mut map: HashMap<i64, i64> = HashMap::new();
    for s in sessions {
        if day_of(s.started_at, tz) == day {
            *map.entry(s.routine_id).or_insert(0) += s.seconds;
        }
    }
    map
}

pub fn today_total(sessions: &[FocusSession], day: NaiveDate, tz: FixedOffset) -> i64 {
    sessions
        .iter()
        .filter(|s| day_of(s.started_at, tz) == day)
        .map(|s| s.seconds)
        .sum()
}

pub fn completed_count(
    routines: &[Routine],
    sessions: &[FocusSession],
    day: NaiveDate,
    tz: FixedOffset,
) -> usize {
    let per = seconds_per_routine(sessions, day, tz);
    routines
        .iter()
        .filter(|r| !r.archived)
        .filter(|r| per.get(&r.id).copied().unwrap_or(0) >= r.target_seconds)
        .count()
}

fn day_qualifies(
    routines: &[Routine],
    sessions: &[FocusSession],
    rule: StreakRule,
    day: NaiveDate,
    tz: FixedOffset,
) -> bool {
    let per = seconds_per_routine(sessions, day, tz);
    match rule {
        StreakRule::Focused => per.values().any(|&s| s > 0),
        StreakRule::AnyCompleted => routines
            .iter()
            .filter(|r| !r.archived)
            .any(|r| per.get(&r.id).copied().unwrap_or(0) >= r.target_seconds),
        StreakRule::AllCompleted => {
            let active: Vec<_> = routines.iter().filter(|r| !r.archived).collect();
            !active.is_empty()
                && active
                    .iter()
                    .all(|r| per.get(&r.id).copied().unwrap_or(0) >= r.target_seconds)
        }
    }
}

pub fn streak(
    routines: &[Routine],
    sessions: &[FocusSession],
    rule: StreakRule,
    today: NaiveDate,
    tz: FixedOffset,
) -> u32 {
    let mut count = 0u32;
    let mut day = today;
    for _ in 0..3650 {
        if day_qualifies(routines, sessions, rule, day, tz) {
            count += 1;
            match day.pred_opt() {
                Some(prev) => day = prev,
                None => break,
            }
        } else {
            break;
        }
    }
    count
}

pub fn max_streak(
    routines: &[Routine],
    sessions: &[FocusSession],
    rule: StreakRule,
    today: NaiveDate,
    tz: FixedOffset,
) -> u32 {
    let earliest = sessions
        .iter()
        .map(|s| day_of(s.started_at, tz))
        .min();
    let earliest = match earliest {
        Some(d) => d,
        None => return 0,
    };

    let mut max_run = 0u32;
    let mut current_run = 0u32;
    let mut day = today;

    loop {
        if day_qualifies(routines, sessions, rule, day, tz) {
            current_run += 1;
            if current_run > max_run {
                max_run = current_run;
            }
        } else {
            current_run = 0;
        }

        if day <= earliest {
            break;
        }
        match day.pred_opt() {
            Some(prev) => day = prev,
            None => break,
        }
    }

    max_run
}

pub fn remaining_total(
    routines: &[Routine],
    sessions: &[FocusSession],
    day: NaiveDate,
    tz: FixedOffset,
) -> i64 {
    let per = seconds_per_routine(sessions, day, tz);
    routines
        .iter()
        .filter(|r| !r.archived)
        .map(|r| {
            let done = per.get(&r.id).copied().unwrap_or(0);
            (r.target_seconds - done).max(0)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::*;
    use chrono::{FixedOffset, TimeZone, Utc};

    fn utc() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    fn sess(routine_id: i64, ts: i64, secs: i64) -> FocusSession {
        let t = Utc.timestamp_opt(ts, 0).unwrap();
        FocusSession { id: 0, routine_id, started_at: t, ended_at: t, seconds: secs, completed: false }
    }

    fn routine(id: i64, target: i64) -> Routine {
        Routine {
            id,
            name: "r".into(),
            icon: "x".into(),
            color: None,
            target_seconds: target,
            pomodoro_enabled: true,
            focus_minutes: 25,
            break_minutes: 5,
            sort_order: id,
            archived: false,
            created_at: "".into(),
        }
    }

    #[test]
    fn sums_and_completion() {
        let day = day_of(Utc.timestamp_opt(1_700_000_000, 0).unwrap(), utc());
        let base = 1_700_000_000;
        let sessions = vec![sess(1, base, 600), sess(1, base + 10, 200), sess(2, base + 20, 100)];
        let per = seconds_per_routine(&sessions, day, utc());
        assert_eq!(per.get(&1), Some(&800));
        assert_eq!(today_total(&sessions, day, utc()), 900);
        let routines = vec![routine(1, 800), routine(2, 4000)];
        assert_eq!(completed_count(&routines, &sessions, day, utc()), 1); // routine 1 met 800
        assert_eq!(remaining_total(&routines, &sessions, day, utc()), 3900); // r2: 4000-100
    }

    #[test]
    fn ignores_other_days() {
        let day = day_of(Utc.timestamp_opt(1_700_000_000, 0).unwrap(), utc());
        let yesterday = 1_700_000_000 - 86_400;
        let sessions = vec![sess(1, yesterday, 999)];
        assert_eq!(today_total(&sessions, day, utc()), 0);
    }

    #[test]
    fn streak_focused_counts_consecutive_days() {
        let utc = utc();
        let today = day_of(Utc.timestamp_opt(1_700_000_000,0).unwrap(), utc);
        let base = 1_700_000_000;
        // today, yesterday, day-before => 3 consecutive; skip day-4
        let sessions = vec![
            sess(1, base, 100),
            sess(1, base - 86_400, 100),
            sess(1, base - 2*86_400, 100),
            sess(1, base - 4*86_400, 100),
        ];
        let routines = vec![routine(1, 50)];
        assert_eq!(streak(&routines, &sessions, StreakRule::Focused, today, utc), 3);
    }

    #[test]
    fn streak_zero_when_today_empty() {
        let utc = utc();
        let today = day_of(Utc.timestamp_opt(1_700_000_000,0).unwrap(), utc);
        let sessions = vec![sess(1, 1_700_000_000 - 86_400, 100)];
        let routines = vec![routine(1, 50)];
        assert_eq!(streak(&routines, &sessions, StreakRule::Focused, today, utc), 0);
    }

    #[test]
    fn streak_all_completed_requires_every_routine() {
        let utc = utc();
        let today = day_of(Utc.timestamp_opt(1_700_000_000,0).unwrap(), utc);
        let base = 1_700_000_000;
        let sessions = vec![sess(1, base, 100), sess(2, base, 100)]; // only r1 meets target today
        let routines = vec![routine(1, 50), routine(2, 4000)];
        assert_eq!(streak(&routines, &sessions, StreakRule::AllCompleted, today, utc), 0);
        assert_eq!(streak(&routines, &sessions, StreakRule::AnyCompleted, today, utc), 1);
    }

    #[test]
    fn max_streak_finds_longest_past_run() {
        let utc = utc();
        let today = day_of(Utc.timestamp_opt(1_700_000_000,0).unwrap(), utc);
        let base = 1_700_000_000;
        // current run of 1 (today only), older run of 3 (days 5,6,7 back)
        let sessions = vec![
            sess(1, base, 100),
            sess(1, base - 5*86_400, 100),
            sess(1, base - 6*86_400, 100),
            sess(1, base - 7*86_400, 100),
        ];
        let routines = vec![routine(1, 50)];
        assert_eq!(streak(&routines, &sessions, StreakRule::Focused, today, utc), 1);
        assert_eq!(max_streak(&routines, &sessions, StreakRule::Focused, today, utc), 3);
    }
}
