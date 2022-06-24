use time_v1::Duration;

/// Formats a [`time_v1::Duration`] into a human-readable string.
pub fn format_time_v1_duration(duration: Duration) -> String {
    duration
        .to_std()
        .map(|x| format!("{}", humantime::format_duration(x)))
        .unwrap_or_else(|_| "<error: negative duration>".to_string())
}
