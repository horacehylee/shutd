use chrono::{DateTime, Datelike, Days, Duration, Local, NaiveTime};

use crate::config::{Config, WeekdayRule};

/// The next shutdown across per-weekday rules. Each rule fires on its
/// weekday's calendar date, shifted one day forward when `nextDay` is set.
/// Scanning the next 9 days covers every weekday at least once; the earliest
/// candidate still ahead wins, so seven identical slots behave exactly like
/// the old daily schedule.
pub fn next_occurrence(
    weekdays: &[WeekdayRule],
    now: DateTime<Local>,
) -> Result<DateTime<Local>, String> {
    if weekdays.len() != 7 {
        return Err("weekdays must have 7 entries".into());
    }
    let times: Vec<NaiveTime> = weekdays
        .iter()
        .map(|d| parse_time(&d.time))
        .collect::<Result<_, _>>()?;
    (0..=8u64)
        .filter_map(|offset| {
            let slot_day = now.date_naive() + Days::new(offset);
            let index = slot_day.weekday().num_days_from_monday() as usize;
            let fire_day = slot_day + Days::new(weekdays[index].next_day as u64);
            fire_day
                .and_time(times[index])
                .and_local_timezone(Local)
                .single()
        })
        .filter(|at| *at > now)
        .min()
        .ok_or_else(|| "no upcoming shutdown".into())
}

pub fn parse_time(start_time: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(start_time, "%H:%M")
        .map_err(|_| format!("invalid time \"{start_time}\", expected HH:MM"))
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
    /// The shutdown time whose toast the user (or a timeout) dismissed;
    /// dismissal is sticky until the shutdown time moves.
    pub dismissed_for: Option<DateTime<Local>>,
}

impl Schedule {
    pub fn new(config: &Config) -> Result<Self, String> {
        Self::new_at(config, Local::now())
    }

    pub fn new_at(config: &Config, now: DateTime<Local>) -> Result<Self, String> {
        Ok(Self {
            shutdown_at: next_occurrence(&config.weekdays, now)?,
            snoozed: 0,
            toast: None,
            dismissed_for: None,
        })
    }

    /// Pushes the shutdown back by the configured snooze interval and clears
    /// the toast so the notification can fire again before the new time.
    pub fn snooze(&mut self, config: &Config) {
        self.shutdown_at += Duration::minutes(i64::from(config.snooze_interval));
        self.snoozed += 1;
        self.toast = None;
        self.dismissed_for = None;
    }

    /// Replaces the schedule after a config change; snooze state resets.
    pub fn reconfigure(&mut self, config: &Config) -> Result<(), String> {
        self.reconfigure_at(config, Local::now())
    }

    pub fn reconfigure_at(&mut self, config: &Config, now: DateTime<Local>) -> Result<(), String> {
        self.shutdown_at = next_occurrence(&config.weekdays, now)?;
        self.snoozed = 0;
        self.toast = None;
        self.dismissed_for = None;
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
        if self.dismissed_for == Some(self.shutdown_at) {
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
                self.dismissed_for = Some(self.shutdown_at);
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
        Config::with_start_time(start)
    }

    #[test]
    fn schedules_later_today() {
        let now = at(2026, 10, 11, 22, 13);
        assert_eq!(
            next_occurrence(&config("01:00").weekdays, now).unwrap(),
            at(2026, 10, 12, 1, 0)
        );
        assert_eq!(
            next_occurrence(&config("23:00").weekdays, now).unwrap(),
            at(2026, 10, 11, 23, 0)
        );
    }

    #[test]
    fn rejects_invalid_time() {
        assert!(next_occurrence(&config("32:00").weekdays, at(2026, 10, 11, 8, 0)).is_err());
        assert!(next_occurrence(&config("ab:cd").weekdays, at(2026, 10, 11, 8, 0)).is_err());
    }

    #[test]
    fn rejects_wrong_weekday_count() {
        assert!(next_occurrence(
            &config("01:00").weekdays[..3].to_vec(),
            at(2026, 10, 11, 8, 0)
        )
        .is_err());
    }

    /// Monday-first slots; every entry is (time, nextDay).
    fn rules(entries: [(&str, bool); 7]) -> Vec<WeekdayRule> {
        entries
            .into_iter()
            .map(|(time, next_day)| WeekdayRule {
                time: time.into(),
                next_day,
            })
            .collect()
    }

    #[test]
    fn cross_day_slot_fires_next_date() {
        // Every weekday's own slot has passed; Monday 01:00 belongs to
        // Monday's session, so it lands Tuesday.
        let weekdays = rules([
            ("01:00", true),
            ("12:00", false),
            ("12:00", false),
            ("12:00", false),
            ("12:00", false),
            ("12:00", false),
            ("12:00", false),
        ]);
        let now = at(2026, 10, 12, 20, 0); // Monday evening
        assert_eq!(
            next_occurrence(&weekdays, now).unwrap(),
            at(2026, 10, 13, 1, 0)
        );
    }

    #[test]
    fn same_day_override_stays_today() {
        let weekdays = rules([
            ("23:00", false),
            ("01:00", true),
            ("01:00", true),
            ("01:00", true),
            ("01:00", true),
            ("01:00", true),
            ("01:00", true),
        ]);
        let now = at(2026, 10, 12, 20, 0); // Monday evening
        assert_eq!(
            next_occurrence(&weekdays, now).unwrap(),
            at(2026, 10, 12, 23, 0)
        );
    }

    #[test]
    fn falls_through_to_next_weekday_slot() {
        let weekdays = rules([
            ("22:00", false),
            ("01:00", true),
            ("02:00", true),
            ("01:00", true),
            ("01:00", true),
            ("01:00", true),
            ("01:00", true),
        ]);
        let now = at(2026, 10, 12, 22, 30); // after Monday's shutdown
                                            // Tuesday's slot crosses days: Wednesday 01:00.
        assert_eq!(
            next_occurrence(&weekdays, now).unwrap(),
            at(2026, 10, 14, 1, 0)
        );
    }

    #[test]
    fn sunday_wraps_to_monday() {
        let weekdays = rules([
            ("23:45", false),
            ("23:45", false),
            ("23:45", false),
            ("23:45", false),
            ("23:45", false),
            ("23:45", false),
            ("23:30", true),
        ]);
        let now = at(2026, 10, 11, 20, 0); // Sunday evening
                                           // Sunday's slot crosses days and lands Monday evening.
        assert_eq!(
            next_occurrence(&weekdays, now).unwrap(),
            at(2026, 10, 12, 23, 30)
        );
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
            dismissed_for: None,
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
            dismissed_for: None,
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
            dismissed_for: None,
        };
        let early = at(2026, 10, 11, 22, 0);
        assert_eq!(schedule.toast_due(&cfg, early), None);

        let due = at(2026, 10, 12, 0, 52);
        let dismiss = schedule.toast_due(&cfg, due).expect("toast is due");
        assert_eq!(dismiss, due + Duration::minutes(10));
        assert_eq!(
            schedule.toast_due(&cfg, at(2026, 10, 12, 0, 56)),
            None,
            "no repeat"
        );

        assert!(!schedule.toast_expired(at(2026, 10, 12, 1, 1)));
        assert!(schedule.toast_expired(at(2026, 10, 12, 1, 2)));
        assert_eq!(
            schedule.toast_due(&cfg, at(2026, 10, 12, 1, 2)),
            None,
            "expiry is sticky"
        );
    }

    #[test]
    fn dismiss_stays_dismissed_until_shutdown_moves() {
        let cfg = config("01:00");
        let mut schedule = Schedule {
            shutdown_at: at(2026, 10, 12, 1, 0),
            snoozed: 0,
            toast: None,
            dismissed_for: None,
        };
        assert!(schedule.toast_due(&cfg, at(2026, 10, 12, 0, 52)).is_some());
        schedule.dismissed_for = Some(schedule.shutdown_at);
        schedule.toast = None;
        assert_eq!(
            schedule.toast_due(&cfg, at(2026, 10, 12, 0, 54)),
            None,
            "dismiss is sticky"
        );

        schedule.snooze(&cfg);
        assert!(
            schedule.toast_due(&cfg, at(2026, 10, 12, 1, 6)).is_some(),
            "snooze re-arms"
        );
    }
}
