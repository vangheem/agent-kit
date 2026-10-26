use agent_kit_core::action::{ActionContext, ActionError, ActionOutput, ServiceKind};
use serde_json::{Map, Number, Value};

pub(crate) fn invalid(message: impl Into<String>) -> ActionError {
    ActionError::InvalidInput {
        message: message.into(),
    }
}

pub(crate) fn ok(body: Value) -> ActionOutput {
    ActionOutput {
        body,
        input_blob_id: None,
        output_blob_id: None,
    }
}

pub(crate) async fn ensure_mock(ctx: &ActionContext, kind: ServiceKind) -> Result<(), ActionError> {
    match ctx.services.connection(ctx.user_id, kind).await? {
        Some(connection) if connection.provider == "mock" => Ok(()),
        Some(connection) => Err(ActionError::ProviderNotConfigured {
            provider: connection.provider,
        }),
        None if ctx.allow_mock => Ok(()),
        None => Err(ActionError::ProviderNotConfigured {
            provider: "none".into(),
        }),
    }
}

pub(crate) fn require_object(input: &Value) -> Result<&Map<String, Value>, ActionError> {
    input
        .as_object()
        .ok_or_else(|| invalid("input must be an object"))
}

pub(crate) fn require_string(input: &Value, field: &str) -> Result<String, ActionError> {
    match input.get(field) {
        Some(Value::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Err(invalid(format!("{field} must not be empty")))
            } else if trimmed.len() > 2_048 {
                Err(invalid(format!("{field} is too long")))
            } else {
                Ok(trimmed.to_owned())
            }
        }
        Some(_) => Err(invalid(format!("{field} must be a string"))),
        None => Err(invalid(format!("{field} is required"))),
    }
}

pub(crate) fn optional_limit(
    input: &Value,
    default: usize,
    max: usize,
) -> Result<usize, ActionError> {
    match input.get("limit") {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(number)) => {
            bounded_u64(number, "limit", 1, max as u64).map(|value| value as usize)
        }
        Some(_) => Err(invalid("limit must be an integer")),
    }
}

pub(crate) fn require_u32(
    input: &Value,
    field: &str,
    min: u32,
    max: u32,
) -> Result<u32, ActionError> {
    match input.get(field) {
        Some(Value::Number(number)) => {
            bounded_u64(number, field, u64::from(min), u64::from(max)).map(|value| value as u32)
        }
        Some(_) => Err(invalid(format!("{field} must be an integer"))),
        None => Err(invalid(format!("{field} is required"))),
    }
}

pub(crate) fn optional_u32(
    input: &Value,
    field: &str,
    default: u32,
    min: u32,
    max: u32,
) -> Result<u32, ActionError> {
    match input.get(field) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(number)) => {
            bounded_u64(number, field, u64::from(min), u64::from(max)).map(|value| value as u32)
        }
        Some(_) => Err(invalid(format!("{field} must be an integer"))),
    }
}

pub(crate) fn require_date(input: &Value, field: &str) -> Result<String, ActionError> {
    let value = require_string(input, field)?;
    if is_iso_date(&value) {
        Ok(value)
    } else {
        Err(invalid(format!("{field} must be YYYY-MM-DD")))
    }
}

pub(crate) fn optional_date(input: &Value, field: &str) -> Result<Option<String>, ActionError> {
    match input.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if is_iso_date(value.trim()) => {
            Ok(Some(value.trim().to_owned()))
        }
        Some(_) => Err(invalid(format!("{field} must be YYYY-MM-DD"))),
    }
}

pub(crate) fn reject_card_numbers(input: &Value) -> Result<(), ActionError> {
    if contains_payment_field(input) || contains_pan(input) {
        Err(invalid("card numbers are not accepted"))
    } else {
        Ok(())
    }
}

pub(crate) fn stable_hash(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(crate) fn cents_for(key: &str) -> u32 {
    499 + (stable_hash(key) % 20_000) as u32
}

pub(crate) fn money(cents: u32) -> Value {
    serde_json::json!({
        "amount": cents_to_number(cents),
        "currency": "USD",
    })
}

pub(crate) fn cents_to_number(cents: u32) -> Value {
    let text = format!("{}.{:02}", cents / 100, cents % 100);
    serde_json::from_str(&text).unwrap_or_else(|_| Value::from(cents / 100))
}

pub(crate) fn hold_id(scope: &str, key: &str) -> String {
    format!("hold_{:016x}", stable_hash(&format!("{scope}:{key}")))
}

fn bounded_u64(number: &Number, field: &str, min: u64, max: u64) -> Result<u64, ActionError> {
    let Some(value) = whole_u64(number) else {
        return Err(invalid(format!("{field} must be an integer")));
    };
    if value < min || value > max {
        return Err(invalid(format!("{field} must be between {min} and {max}")));
    }
    Ok(value)
}

fn whole_u64(number: &Number) -> Option<u64> {
    if let Some(value) = number.as_u64() {
        return Some(value);
    }
    let value = number.as_f64()?;
    if value.is_finite() && value.fract() == 0.0 && (0.0..=(u64::MAX as f64)).contains(&value) {
        Some(value as u64)
    } else {
        None
    }
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u16>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return false;
    };
    if year < 1900 {
        return false;
    }
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

fn is_leap(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn contains_payment_field(value: &Value) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(key, child)| payment_key(key) || contains_payment_field(child)),
        Value::Array(items) => items.iter().any(contains_payment_field),
        _ => false,
    }
}

fn payment_key(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect();
    matches!(
        normalized.as_str(),
        "card"
            | "cardnumber"
            | "cardnum"
            | "creditcard"
            | "pan"
            | "cvv"
            | "cvc"
            | "cvc2"
            | "payment"
            | "paymentmethod"
            | "paymenttoken"
            | "cardtoken"
    ) || normalized.contains("cardnumber")
        || normalized.contains("creditcard")
}

fn contains_pan(value: &Value) -> bool {
    match value {
        Value::String(text) => looks_like_pan(text),
        Value::Object(map) => map.values().any(contains_pan),
        Value::Array(items) => items.iter().any(contains_pan),
        _ => false,
    }
}

fn looks_like_pan(value: &str) -> bool {
    if value
        .chars()
        .any(|ch| !(ch.is_ascii_digit() || ch == ' ' || ch == '-'))
    {
        return false;
    }
    let digits: String = value.chars().filter(|ch| ch.is_ascii_digit()).collect();
    (13..=19).contains(&digits.len()) && luhn(&digits)
}

fn luhn(digits: &str) -> bool {
    let mut sum = 0;
    let mut alternate = false;
    for ch in digits.chars().rev() {
        let Some(mut digit) = ch.to_digit(10) else {
            return false;
        };
        if alternate {
            digit *= 2;
            if digit > 9 {
                digit -= 9;
            }
        }
        sum += digit;
        alternate = !alternate;
    }
    sum % 10 == 0
}
