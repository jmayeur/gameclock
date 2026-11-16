//! The tables that the config gives: the schedule, the alerts, and the
//! trigger rules (`openspec/specs/daemon-config`, docs/adr/0019).

use crate::alert::{Alerts, Trigger};
use crate::schedule::Schedule;

/// The schedule, the alert table, and the trigger rules that the daemon uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tables {
    /// The color and brightness for each local time.
    pub schedule: Schedule,
    /// The alerts that the `alert` command and the triggers can start.
    pub alerts: Alerts,
    /// The rules that start alerts at fixed times.
    pub triggers: Vec<Trigger>,
}

impl Tables {
    /// The built-in tables: the legacy schedule, the built-in alerts, and the
    /// legacy bedtime triggers.
    #[must_use]
    pub fn legacy() -> Self {
        let alerts = Alerts::legacy();
        let triggers = Trigger::legacy(&alerts);
        Self {
            schedule: Schedule::legacy(),
            alerts,
            triggers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_tables() {
        let t = Tables::legacy();
        assert_eq!(t.schedule, Schedule::legacy());
        assert_eq!(t.alerts, Alerts::legacy());
        assert_eq!(t.triggers, Trigger::legacy(&Alerts::legacy()));
        let names: Vec<_> = t.triggers.iter().map(|r| r.alert.name.as_str()).collect();
        assert_eq!(names, ["bed-flash", "bedtime"]);
        assert_eq!(Tables::legacy(), Tables::legacy());
    }
}
