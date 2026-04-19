use serde::Deserialize;
use serde_json::Value;
use time::OffsetDateTime;

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
        Value::Null | Value::String(_) => Ok(None),
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

pub fn deserialize_null_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::String(s) => {
            if s == "null" {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        }
        Value::Array(v) => v
            .first()
            .and_then(|v| v.as_str())
            .filter(|s| *s != "null")
            .map(|s| Ok(Some(String::from(s))))
            .unwrap_or_else(|| Err(serde::de::Error::custom("invalid array of strings"))),
        Value::Null => Ok(None),
        _ => Err(serde::de::Error::custom("expected string or null")),
    }
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

fn extract_url(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => (!s.is_empty() && s != "null").then(|| s.clone()),
        Value::Object(map) => {
            if let Some(url) = map.get("url").and_then(Value::as_str) {
                return (!url.is_empty()).then(|| url.to_string());
            }

            map.values().find_map(extract_url)
        }
        Value::Array(items) => items.iter().find_map(extract_url),
        _ => None,
    }
}

pub fn deserialize_null_screenshot<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Value::deserialize(deserializer)? {
        Value::Null => Ok(None),
        value => Ok(extract_url(&value)),
    }
}
