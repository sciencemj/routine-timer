use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Mode {
    Pomodoro,
    Continuous,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum StreakRule {
    Focused,
    AnyCompleted,
    AllCompleted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Routine {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub color: Option<String>,
    pub target_seconds: i64,
    pub pomodoro_enabled: bool,
    pub focus_minutes: i64,
    pub break_minutes: i64,
    pub sort_order: i64,
    pub archived: bool,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewRoutine {
    pub name: String,
    pub icon: String,
    pub color: Option<String>,
    pub target_seconds: i64,
    pub pomodoro_enabled: bool,
    pub focus_minutes: i64,
    pub break_minutes: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FocusSession {
    pub id: i64,
    pub routine_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub seconds: i64,
    pub completed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mode_serializes_to_variant_name() {
        assert_eq!(serde_json::to_string(&Mode::Pomodoro).unwrap(), "\"Pomodoro\"");
        assert_eq!(serde_json::to_string(&StreakRule::AllCompleted).unwrap(), "\"AllCompleted\"");
    }
}
