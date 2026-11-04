use chrono::{DateTime, Utc};
use cowbot_core::error::{CowError, CowResult};

pub(crate) fn db(err: sqlx::Error) -> CowError {
    CowError::Storage(err.to_string())
}

pub(crate) fn parse_datetime(value: &str) -> CowResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| CowError::Storage(format!("invalid timestamp '{value}': {err}")))
}

pub(crate) fn parse_optional_datetime(value: Option<&str>) -> CowResult<Option<DateTime<Utc>>> {
    value.map(parse_datetime).transpose()
}

pub(crate) fn parse_json<T: serde::de::DeserializeOwned>(value: &str, field: &str) -> CowResult<T> {
    serde_json::from_str(value)
        .map_err(|err| CowError::Storage(format!("invalid JSON in '{field}': {err}")))
}
