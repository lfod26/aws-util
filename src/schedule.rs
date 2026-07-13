use anyhow::{Context, Result};
use chrono::{Local, NaiveTime, TimeDelta};

/// Parses a "HH:MM" 24-hour local time string and computes how many
/// whole minutes remain until the next occurrence of that clock time
/// (today if it hasn't passed yet, otherwise tomorrow).
///
/// Returns `(minutes, target_time)` so callers can print a friendly
/// message including the resolved target time.
pub fn minutes_until_next(time_str: &str) -> Result<(i64, NaiveTime)> {
    let target_time = NaiveTime::parse_from_str(time_str, "%H:%M").with_context(|| {
        format!("invalid time '{time_str}', expected 24-hour HH:MM (e.g. 18:30)")
    })?;

    let now = Local::now();
    let today_target = now.date_naive().and_time(target_time);

    let target = if today_target > now.naive_local() {
        today_target
    } else {
        today_target + TimeDelta::days(1)
    };

    let minutes = (target - now.naive_local()).num_minutes();
    // Round up to make sure we don't schedule for a moment slightly
    // before the target time due to truncation.
    let minutes = minutes.max(1);

    Ok((minutes, target_time))
}
