use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::clock::Clock;
use crate::core::model::Mode;

// ── State / phase / event enums ────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum TimerState {
    Idle,
    Running,
    Paused,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Phase {
    Focus,
    Break,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum TimerEvent {
    FocusEnded,
    BreakEnded,
    TargetReached,
}

// ── Config / snapshot / completed-session ──────────────────────────────────

/// Suspended pomodoro state persisted per-routine so an interrupted pomodoro
/// block RESUMES (remaining time, phase, session index) instead of restarting.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResumeState {
    pub remaining_secs: i64,
    pub phase: Phase,
    pub pomodoro_index: u32,
}

pub struct TimerConfig {
    pub routine_id: i64,
    pub mode: Mode,
    pub focus_secs: i64,
    pub break_secs: i64,
    pub target_secs: i64,
    pub already_done_secs: i64,
    /// When Some and mode is Pomodoro, `start` resumes from this state instead
    /// of a fresh Focus interval. Ignored for Continuous mode.
    pub resume: Option<ResumeState>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TimerSnapshot {
    pub state: TimerState,
    pub mode: Mode,
    pub phase: Phase,
    pub routine_id: Option<i64>,
    pub pomodoro_index: u32,
    pub remaining_secs: i64,
    pub session_seconds: i64,
    pub routine_today_secs: i64,
    pub target_secs: i64,
    pub state_changed: bool,
    pub event: Option<TimerEvent>,
    pub remaining_label: String,
}

pub struct CompletedSession {
    pub routine_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub seconds: i64,
    pub completed: bool,
}

// ── TimerEngine ────────────────────────────────────────────────────────────

pub struct TimerEngine {
    clock: Box<dyn Clock>,
    state: TimerState,
    mode: Mode,
    phase: Phase,
    routine_id: Option<i64>,
    pomodoro_index: u32,
    remaining: i64,
    session_focus_secs: i64,
    already_done_secs: i64,
    target_secs: i64,
    focus_secs: i64,
    break_secs: i64,
    started_at: Option<DateTime<Utc>>,
    pending_completed: Option<CompletedSession>,
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Format seconds as mm:ss, or h:mm:ss when >= 1 hour.
fn label(secs: i64) -> String {
    let s = secs.max(0);
    let hours = s / 3600;
    let mins = (s % 3600) / 60;
    let sec = s % 60;
    if hours > 0 {
        format!("{:01}:{:02}:{:02}", hours, mins, sec)
    } else {
        format!("{:02}:{:02}", mins, sec)
    }
}

// ── Impl ───────────────────────────────────────────────────────────────────

impl TimerEngine {
    /// Create an engine in the Idle state.
    pub fn new(clock: Box<dyn Clock>) -> Self {
        TimerEngine {
            clock,
            state: TimerState::Idle,
            mode: Mode::Continuous,
            phase: Phase::Focus,
            routine_id: None,
            pomodoro_index: 0,
            remaining: 0,
            session_focus_secs: 0,
            already_done_secs: 0,
            target_secs: 0,
            focus_secs: 0,
            break_secs: 0,
            started_at: None,
            pending_completed: None,
        }
    }

    /// Transition from Idle to Running (Focus phase).
    pub fn start(&mut self, cfg: TimerConfig) {
        self.mode = cfg.mode;
        self.routine_id = Some(cfg.routine_id);
        self.focus_secs = cfg.focus_secs;
        self.break_secs = cfg.break_secs;
        self.target_secs = cfg.target_secs;
        self.already_done_secs = cfg.already_done_secs;
        self.session_focus_secs = 0;
        self.phase = Phase::Focus;
        self.state = TimerState::Running;
        self.started_at = Some(self.clock.now());
        self.pending_completed = None;
        match cfg.mode {
            Mode::Pomodoro => match cfg.resume {
                // RESUME: restore the suspended block (phase, remaining, index).
                Some(rs) => {
                    self.phase = rs.phase;
                    self.remaining = rs.remaining_secs;
                    self.pomodoro_index = rs.pomodoro_index;
                }
                // Fresh start: begin at Focus interval 1.
                None => {
                    self.pomodoro_index = 1;
                    self.remaining = cfg.focus_secs;
                }
            },
            Mode::Continuous => {
                self.pomodoro_index = 0;
                self.remaining = cfg.target_secs - cfg.already_done_secs;
            }
        }
    }

    /// Advance exactly 1 second. Pure counter — does NOT diff wall-clock time.
    /// Clock is used only for `ended_at` on auto-finalize.
    pub fn tick(&mut self) -> TimerSnapshot {
        let prev_state = self.state;
        let mut event: Option<TimerEvent> = None;
        let mut phase_changed = false;

        if self.state == TimerState::Running {
            match self.phase {
                Phase::Focus => {
                    if self.remaining > 0 {
                        self.remaining -= 1;
                        self.session_focus_secs += 1;
                    }

                    // The routine's required time is filled → finalize to Idle
                    // (BOTH modes). For Continuous this coincides with remaining==0;
                    // for Pomodoro this stops the focus/break cycling once the daily
                    // target is met, instead of looping forever.
                    let target_filled = self.target_secs > 0
                        && self.already_done_secs + self.session_focus_secs >= self.target_secs;

                    if target_filled {
                        self.state = TimerState::Idle;
                        event = Some(TimerEvent::TargetReached);
                        self.pending_completed = Some(CompletedSession {
                            routine_id: self.routine_id.unwrap(),
                            started_at: self.started_at.unwrap(),
                            ended_at: self.clock.now(),
                            seconds: self.session_focus_secs,
                            completed: true,
                        });
                    } else if self.mode == Mode::Pomodoro && self.remaining == 0 {
                        // Focus block ended before the target → switch to break.
                        self.phase = Phase::Break;
                        self.remaining = self.break_secs;
                        event = Some(TimerEvent::FocusEnded);
                        phase_changed = true;
                    }
                }
                Phase::Break => {
                    // Break ticks decrement remaining but do NOT add focus seconds.
                    if self.remaining > 0 {
                        self.remaining -= 1;
                    }

                    if self.remaining == 0 {
                        // Switch back to next focus interval.
                        self.phase = Phase::Focus;
                        self.remaining = self.focus_secs;
                        self.pomodoro_index += 1;
                        event = Some(TimerEvent::BreakEnded);
                        phase_changed = true;
                    }
                }
            }
        }

        let state_changed = phase_changed || (self.state != prev_state);
        self.build_snapshot(event, state_changed)
    }

    /// Return the current state without advancing time.
    pub fn snapshot(&self) -> TimerSnapshot {
        self.build_snapshot(None, false)
    }

    pub fn state(&self) -> TimerState {
        self.state
    }

    /// Pause a running session. Running → Paused; freezes the countdown.
    pub fn pause(&mut self) {
        if self.state == TimerState::Running {
            self.state = TimerState::Paused;
        }
    }

    /// Resume from Paused back to Running.
    pub fn resume(&mut self) {
        if self.state == TimerState::Paused {
            self.state = TimerState::Running;
        }
    }

    /// Skip the current break and jump immediately to the next focus interval.
    /// Only meaningful during Pomodoro break phase; a no-op otherwise.
    pub fn skip_break(&mut self) {
        if self.phase == Phase::Break {
            self.phase = Phase::Focus;
            self.pomodoro_index += 1;
            self.remaining = self.focus_secs;
        }
    }

    /// Manually stop the timer and return a `CompletedSession`.
    ///
    /// - If a session is active (Running/Paused): builds the session, resets to Idle, returns Some.
    /// - If Idle with a pending auto-finalized session: drains and returns it.
    /// - If Idle with nothing pending: returns None.
    pub fn stop(&mut self) -> Option<CompletedSession> {
        match self.state {
            TimerState::Idle => {
                // Drain any auto-finalized session (e.g. continuous mode hit target).
                self.pending_completed.take()
            }
            _ => {
                let done = CompletedSession {
                    routine_id: self.routine_id.unwrap(),
                    started_at: self.started_at.unwrap(),
                    ended_at: self.clock.now(),
                    seconds: self.session_focus_secs,
                    completed: self.already_done_secs + self.session_focus_secs >= self.target_secs,
                };
                self.state = TimerState::Idle;
                self.started_at = None;
                self.pending_completed = None;
                Some(done)
            }
        }
    }

    /// Drain the stashed CompletedSession (set by auto-finalize). Returns None if empty.
    pub fn take_completed(&mut self) -> Option<CompletedSession> {
        self.pending_completed.take()
    }

    // ── Private ────────────────────────────────────────────────────────────

    fn build_snapshot(&self, event: Option<TimerEvent>, state_changed: bool) -> TimerSnapshot {
        let remaining_secs = self.remaining.max(0);
        TimerSnapshot {
            state: self.state,
            mode: self.mode,
            phase: self.phase,
            routine_id: self.routine_id,
            pomodoro_index: self.pomodoro_index,
            remaining_secs,
            session_seconds: self.session_focus_secs,
            routine_today_secs: self.already_done_secs + self.session_focus_secs,
            target_secs: self.target_secs,
            state_changed,
            event,
            remaining_label: label(remaining_secs),
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::clock::Clock;
    use chrono::{DateTime, Utc, TimeZone};

    struct FakeClock(DateTime<Utc>);
    impl Clock for FakeClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn engine() -> TimerEngine {
        TimerEngine::new(Box::new(FakeClock(Utc.timestamp_opt(1_700_000_000, 0).unwrap())))
    }

    fn cont_cfg() -> TimerConfig {
        TimerConfig {
            routine_id: 1,
            mode: Mode::Continuous,
            focus_secs: 0,
            break_secs: 0,
            target_secs: 60,
            already_done_secs: 0,
            resume: None,
        }
    }

    #[test]
    fn idle_tick_is_noop() {
        let mut e = engine();
        let snap = e.tick();
        assert_eq!(snap.state, TimerState::Idle);
        assert_eq!(snap.remaining_secs, 0);
    }

    #[test]
    fn continuous_counts_down_and_labels() {
        let mut e = engine();
        e.start(cont_cfg());
        let s = e.snapshot();
        assert_eq!(s.state, TimerState::Running);
        assert_eq!(s.remaining_secs, 60);
        assert_eq!(s.remaining_label, "01:00");
        let s = e.tick();
        assert_eq!(s.remaining_secs, 59);
        assert_eq!(s.session_seconds, 1);
        assert_eq!(s.routine_today_secs, 1);
    }

    #[test]
    fn pomodoro_cycles_focus_break_and_counts_index() {
        let mut e = TimerEngine::new(Box::new(FakeClock(Utc.timestamp_opt(1_700_000_000,0).unwrap())));
        e.start(TimerConfig { routine_id: 1, mode: Mode::Pomodoro, focus_secs: 2, break_secs: 1, target_secs: 3600, already_done_secs: 0, resume: None });
        assert_eq!(e.snapshot().phase, Phase::Focus);
        assert_eq!(e.snapshot().pomodoro_index, 1);
        e.tick();                              // focus 1 left
        let s = e.tick();                      // focus 0 -> break begins
        assert_eq!(s.event, Some(TimerEvent::FocusEnded));
        assert_eq!(s.phase, Phase::Break);
        assert_eq!(s.remaining_secs, 1);
        assert_eq!(s.session_seconds, 2);      // break does not add focus seconds
        let s = e.tick();                      // break 0 -> next focus
        assert_eq!(s.event, Some(TimerEvent::BreakEnded));
        assert_eq!(s.phase, Phase::Focus);
        assert_eq!(s.pomodoro_index, 2);
        assert_eq!(s.remaining_secs, 2);
    }
    #[test]
    fn pomodoro_break_does_not_add_focus_seconds() {
        let mut e = TimerEngine::new(Box::new(FakeClock(Utc.timestamp_opt(1_700_000_000,0).unwrap())));
        e.start(TimerConfig { routine_id: 1, mode: Mode::Pomodoro, focus_secs: 1, break_secs: 2, target_secs: 3600, already_done_secs: 10, resume: None });
        e.tick();                              // focus done -> break
        let s = e.tick();                      // in break
        assert_eq!(s.session_seconds, 1);
        assert_eq!(s.routine_today_secs, 11);  // 10 already + 1 focus
    }

    #[test]
    fn pause_freezes_and_resume_continues() {
        let mut e = engine(); e.start(cont_cfg());
        e.tick();                       // 59
        e.pause();
        assert_eq!(e.state(), TimerState::Paused);
        let s = e.tick();               // paused: no change
        assert_eq!(s.remaining_secs, 59);
        e.resume();
        let s = e.tick();               // 58
        assert_eq!(s.remaining_secs, 58);
    }
    #[test]
    fn stop_returns_completed_session_and_resets() {
        let start_at = Utc.timestamp_opt(1_700_000_000,0).unwrap();
        let mut e = TimerEngine::new(Box::new(FakeClock(start_at)));
        e.start(TimerConfig { target_secs: 3, ..cont_cfg() });
        e.tick(); e.tick(); e.tick();   // 3 focus seconds, target reached
        let done = e.stop().unwrap();
        assert_eq!(done.routine_id, 1);
        assert_eq!(done.seconds, 3);
        assert!(done.completed);
        assert_eq!(done.started_at, start_at);
        assert_eq!(e.state(), TimerState::Idle);
        assert!(e.stop().is_none());     // idle stop -> None
    }
    #[test]
    fn skip_break_jumps_to_next_focus() {
        let mut e = TimerEngine::new(Box::new(FakeClock(Utc.timestamp_opt(1_700_000_000,0).unwrap())));
        e.start(TimerConfig { routine_id: 1, mode: Mode::Pomodoro, focus_secs: 1, break_secs: 30, target_secs: 3600, already_done_secs: 0, resume: None });
        e.tick();                        // -> Break (30 left)
        e.skip_break();
        let s = e.snapshot();
        assert_eq!(s.phase, Phase::Focus);
        assert_eq!(s.pomodoro_index, 2);
        assert_eq!(s.remaining_secs, 1);
    }

    #[test]
    fn continuous_target_reached_finalizes_to_idle_and_records() {
        let start_at = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let mut e = TimerEngine::new(Box::new(FakeClock(start_at)));
        e.start(TimerConfig { target_secs: 2, ..cont_cfg() });
        e.tick(); // 1 left
        let s = e.tick(); // 0 -> auto-finalize
        assert_eq!(s.remaining_secs, 0);
        assert_eq!(s.state, TimerState::Idle);
        assert_eq!(s.event, Some(TimerEvent::TargetReached));
        let done = e.take_completed().unwrap();
        assert_eq!(done.seconds, 2);
        assert!(done.completed);
        assert_eq!(done.started_at, start_at);
        let s = e.tick(); // idle no-op afterwards
        assert_eq!(s.state, TimerState::Idle);
        assert_eq!(s.event, None);
        assert!(e.take_completed().is_none()); // drained
    }

    #[test]
    fn pomodoro_resume_restores_block() {
        // With a resume state, start() picks up mid-block instead of at 1500s / index 1.
        let mut e = engine();
        e.start(TimerConfig {
            routine_id: 1, mode: Mode::Pomodoro, focus_secs: 1500, break_secs: 300,
            target_secs: 3600, already_done_secs: 0,
            resume: Some(ResumeState { remaining_secs: 700, phase: Phase::Focus, pomodoro_index: 3 }),
        });
        let s = e.snapshot();
        assert_eq!(s.remaining_secs, 700);
        assert_eq!(s.pomodoro_index, 3);
        assert_eq!(s.phase, Phase::Focus);

        // Without a resume state, start() begins fresh (focus_secs, index 1).
        let mut e = engine();
        e.start(TimerConfig {
            routine_id: 1, mode: Mode::Pomodoro, focus_secs: 1500, break_secs: 300,
            target_secs: 3600, already_done_secs: 0, resume: None,
        });
        let s = e.snapshot();
        assert_eq!(s.remaining_secs, 1500);
        assert_eq!(s.pomodoro_index, 1);
        assert_eq!(s.phase, Phase::Focus);
    }

    #[test]
    fn pomodoro_finalizes_when_target_filled() {
        // Target fills mid focus-block (before the block ends): pomodoro must stop,
        // not keep cycling focus/break forever.
        let mut e = engine();
        e.start(TimerConfig {
            routine_id: 1, mode: Mode::Pomodoro, focus_secs: 10, break_secs: 5,
            target_secs: 2, already_done_secs: 0, resume: None,
        });
        e.tick();                 // session 1s
        let s = e.tick();         // session 2s -> target filled
        assert_eq!(s.state, TimerState::Idle);
        assert_eq!(s.event, Some(TimerEvent::TargetReached));
        assert_eq!(s.phase, Phase::Focus);       // did NOT flip to break
        let done = e.take_completed().unwrap();
        assert_eq!(done.seconds, 2);
        assert!(done.completed);
    }
}
