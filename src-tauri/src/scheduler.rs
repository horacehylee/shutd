use chrono::{DateTime, Duration, Local, NaiveTime};

use crate::config::Config;

/// The next shutdown occurrence for a daily HH:MM schedule: today when the
/// time is still ahead, otherwise tomorrow.
pub fn next_occurrence(start_time: &str, now: DateTime<Local>) -> Result<DateTime<Local>, String> {
    let time = parse_time(start_time)?;
    let today = now
        .date_naive()
        .and_time(time)
        .and_local_timezone(Local)
        .single()
        .ok_or("invalid local time")?;
    if today > now {
        Ok(today)
    } else {
        Ok(today + Duration::days(1))
    }
}

pub fn parse_time(start_time: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(start_time, "%H:%M").map_err(|_| format!("invalid time \"{start_time}\", expected HH:MM"))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToastState {
    /// The shutdown time this toast was shown for; a snooze or config change
    /// moves the shutdown time, which allows a fresh toast later.
    pub shown_for: DateTime<Local>,
    pub dismiss_at: DateTime<Local>,
}

#[derive(Debug, Clone, Copy)]
pub struct Schedule {
    pub shutdown_at: DateTime<Local>,
    pub snoozed: u32,
    pub toast: Option<ToastState>,
}

impl Schedule {
    pub fn new(config: &Config) -> Result<Self, String> {
        Self::new_at(config, Local::now())
    }

    pub fn new_at(config: &Config, now: DateTime<Local>) -> Result<Self, String> {
        Ok(Self {
            shutdown_at: next_occurrence(&config.start_time, now)?,
            snoozed: 0,
            toast: None,
        })
    }

    /// Pushes the shutdown back by the configured snooze interval and clears
    /// the toast so the notification can fire again before the new time.
    pub fn snooze(&mut self, config: &Config) {
        self.shutdown_at += Duration::minutes(i64::from(config.snooze_interval));
        self.snoozed += 1;
        self.toast = None;
    }

    /// Replaces the schedule after a config change; snooze state resets.
    pub fn reconfigure(&mut self, config: &Config) -> Result<(), String> {
        self.reconfigure_at(config, Local::now())
    }

    pub fn reconfigure_at(&mut self, config: &Config, now: DateTime<Local>) -> Result<(), String> {
        self.shutdown_at = next_occurrence(&config.start_time, now)?;
        self.snoozed = 0;
        self.toast = None;
        Ok(())
    }

    pub fn notify_at(&self, config: &Config) -> DateTime<Local> {
        self.shutdown_at - Duration::minutes(i64::from(config.notification.before))
    }

    /// Shows the toast once per shutdown time inside the notify window.
    pub fn toast_due(&mut self, config: &Config, now: DateTime<Local>) -> Option<DateTime<Local>> {
        if now < self.notify_at(config) || now >= self.shutdown_at {
            return None;
        }
        if self.toast.map(|t| t.shown_for) == Some(self.shutdown_at) {
            return None;
        }
        let dismiss_at = now + Duration::minutes(i64::from(config.notification.duration));
        self.toast = Some(ToastState {
            shown_for: self.shutdown_at,
            dismiss_at,
        });
        Some(dismiss_at)
    }

    /// Expires the toast with no interaction; shutdown then proceeds.
    pub fn toast_expired(&mut self, now: DateTime<Local>) -> bool {
        match self.toast {
            Some(t) if now >= t.dismiss_at => {
                self.toast = None;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    fn config(start: &str) -> Config {
        Config {
            start_time: start.into(),
            ..Config::default()
        }
    }

    #[test]
    fn schedules_later_today() {
        let now = at(2026, 10, 11, 22, 13);
        assert_eq!(next_occurrence("01:00", now).unwrap(), at(2026, 10, 12, 1, 0));
        assert_eq!(next_occurrence("23:00", now).unwrap(), at(2026, 10, 11, 23, 0));
    }

    #[test]
    fn rejects_invalid_time() {
        assert!(next_occurrence("32:00", at(2026, 10, 11, 8, 0)).is_err());
        assert!(next_occurrence("ab:cd", at(2026, 10, 11, 8, 0)).is_err());
    }

    #[test]
    fn snooze_pushes_shutdown_and_rearms_toast() {
        let cfg = config("01:00");
        let mut schedule = Schedule {
            shutdown_at: at(2026, 10, 12, 1, 0),
            snoozed: 0,
            toast: Some(ToastState {
                shown_for: at(2026, 10, 12, 1, 0),
                dismiss_at: at(2026, 10, 12, 0, 59),
            }),
        };
        schedule.snooze(&cfg);
        assert_eq!(schedule.shutdown_at, at(2026, 10, 12, 1, 15));
        assert_eq!(schedule.snoozed, 1);
        assert_eq!(schedule.toast, None);
    }

    #[test]
    fn reconfigure_resets_snooze_state() {
        let cfg = config("02:00");
        let mut schedule = Schedule {
            shutdown_at: at(2026, 10, 12, 1, 15),
            snoozed: 2,
            toast: None,
        };
        let now = at(2026, 10, 11, 8, 0);
        schedule.reconfigure_at(&cfg, now).unwrap();
        assert_eq!(schedule.shutdown_at, at(2026, 10, 12, 2, 0));
        assert_eq!(schedule.snoozed, 0);
        assert_eq!(schedule.toast, None);
    }

    #[test]
    fn toast_fires_once_inside_window_and_expires() {
        let cfg = config("01:00"); // notify 10 min before
        let mut schedule = Schedule {
            shutdown_at: at(2026, 10, 12, 1, 0),
            snoozed: 0,
            toast: None,
        };
        let early = at(2026, 10, 11, 22, 0);
        assert_eq!(schedule.toast_due(&cfg, early), None);

        let due = at(2026, 10, 12, 0, 52);
        let dismiss = schedule.toast_due(&cfg, due).expect("toast is due");
        assert_eq!(dismiss, due + Duration::minutes(10));
        assert_eq!(schedule.toast_due(&cfg, at(2026, 10, 12, 0, 56)), None, "no repeat");

        assert!(!schedule.toast_expired(at(2026, 10, 12, 1, 1)));
        assert!(schedule.toast_expired(at(2026, 10, 12, 1, 2)));
    }

}
