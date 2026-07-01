use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use std::collections::HashMap;

use crate::core::model::{FocusSession, Routine};

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
}
