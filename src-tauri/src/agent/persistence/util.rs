use chrono::{DateTime, TimeZone, Utc};
use tracing::warn;

/// Represent time as unix timestamp in seconds to match the SQLite schema.
pub(crate) fn now_timestamp() -> i64 {
    Utc::now().timestamp()
}

pub(crate) fn datetime_to_timestamp(dt: DateTime<Utc>) -> i64 {
    dt.timestamp()
}

pub(crate) fn opt_datetime_to_timestamp(dt: Option<DateTime<Utc>>) -> Option<i64> {
    dt.map(datetime_to_timestamp)
}

pub(crate) fn timestamp_to_datetime(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(ts, 0).single().unwrap_or_else(|| {
        warn!("Invalid UTC timestamp '{}', falling back to UNIX_EPOCH", ts);
        DateTime::<Utc>::from(std::time::UNIX_EPOCH)
    })
}

pub(crate) fn opt_timestamp_to_datetime(ts: Option<i64>) -> Option<DateTime<Utc>> {
    ts.map(timestamp_to_datetime)
}

pub(crate) fn bool_to_sql(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}
