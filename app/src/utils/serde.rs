use serde::Deserialize;
use serde_json::Value;
use time::{Date, OffsetDateTime, Time};

pub fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::Number(n) => {
            let ts = n
                .as_i64()
                .ok_or_else(|| serde::de::Error::custom("invalid timestamp"))?;
            OffsetDateTime::from_unix_timestamp(ts)
                .map(Some)
                .map_err(serde::de::Error::custom)
        }
        Value::String(s) => {
            if s.is_empty() || s == "null" {
                Ok(None)
            } else {
                Date::parse(&s, &time::format_description::well_known::Iso8601::DATE)
                    .map(|d| Some(d.with_time(Time::MIDNIGHT).assume_utc()))
                    .map_err(serde::de::Error::custom)
            }
        }
        Value::Null => Ok(None),
        _ => Err(serde::de::Error::custom("expected number, null, or string")),
    }
}

pub fn deserialize_null_int<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::Number(n) => n
            .as_i64()
            .map(|v| Some(v as i32))
            .ok_or_else(|| serde::de::Error::custom("invalid number")),
        Value::Null | Value::String(_) => Ok(None),
        _ => Err(serde::de::Error::custom("expected number, null, or string")),
    }
}

/// Postgres text columns reject 0x00
fn strip_null_bytes(s: String) -> String {
    if s.contains('\0') {
        s.replace('\0', "")
    } else {
        s
    }
}

pub fn deserialize_null_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::String(s) => {
            if s == "null" {
                Ok(None)
            } else {
                Ok(Some(strip_null_bytes(s)))
            }
        }
        Value::Array(v) => Ok(v
            .first()
            .and_then(|v| v.as_str())
            .filter(|s| *s != "null")
            .map(|s| strip_null_bytes(s.to_string()))),
        Value::Null => Ok(None),
        _ => Err(serde::de::Error::custom("expected string or null")),
    }
}

pub fn deserialize_sanitized_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer).map(strip_null_bytes)
}

pub fn deserialize_sanitized_opt_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(|opt| opt.map(strip_null_bytes))
}

pub fn deserialize_null_float<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::Number(n) => n
            .as_f64()
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("invalid number")),
        Value::Array(v) => v
            .first()
            .and_then(|v| v.as_f64())
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom("invalid array of numbers")),
        Value::Null | Value::String(_) => Ok(None),
        _ => Err(serde::de::Error::custom("expected number, null, or string")),
    }
}
