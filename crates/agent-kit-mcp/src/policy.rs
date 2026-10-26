use agent_kit_core::action::Effect;
use serde_json::Value;

pub(crate) fn action_allowed(enabled: &[String], action_id: &str) -> bool {
    enabled.is_empty() || enabled.iter().any(|id| id == action_id)
}

pub(crate) fn search_limit(limit: Option<i64>) -> usize {
    let Some(limit) = limit else {
        return 10;
    };
    usize::try_from(limit).map(|n| n.clamp(1, 50)).unwrap_or(1)
}

pub(crate) fn is_confirmed(confirm: Option<bool>, input: &Value) -> bool {
    confirm == Some(true) || input.get("confirm").and_then(Value::as_bool) == Some(true)
}

pub(crate) fn confirmation_required(effect: &Effect, confirm: Option<bool>, input: &Value) -> bool {
    matches!(effect, Effect::Mutating) && !is_confirmed(confirm, input)
}

pub(crate) fn error_text(code: &str, detail: impl std::fmt::Display) -> String {
    let detail = detail.to_string();
    if detail.starts_with(code) {
        detail
    } else if detail.is_empty() {
        code.to_string()
    } else {
        format!("{code}: {detail}")
    }
}

pub(crate) fn strip_confirm(mut input: Value) -> Value {
    if let Some(object) = input.as_object_mut() {
        object.remove("confirm");
    }
    input
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn confirm_and_enabled_actions() {
        assert!(action_allowed(&[], "markets.order"));
        assert!(action_allowed(
            &["markets.order".to_string()],
            "markets.order"
        ));
        assert!(!action_allowed(
            &["markets.quote".to_string()],
            "markets.order"
        ));

        let empty = json!({});
        let confirmed = json!({"confirm": true});
        assert!(confirmation_required(&Effect::Mutating, None, &empty));
        assert!(!confirmation_required(
            &Effect::Mutating,
            Some(true),
            &empty
        ));
        assert!(!confirmation_required(
            &Effect::Mutating,
            Some(false),
            &confirmed
        ));
        assert!(!confirmation_required(&Effect::Mutating, None, &confirmed));
        assert!(!confirmation_required(&Effect::Read, None, &empty));
        assert!(confirmation_required(
            &Effect::Mutating,
            Some(false),
            &json!({"confirm": false})
        ));
        assert_eq!(search_limit(None), 10);
        assert_eq!(search_limit(Some(0)), 1);
        assert_eq!(search_limit(Some(51)), 50);
        assert_eq!(search_limit(Some(7)), 7);
    }
}
