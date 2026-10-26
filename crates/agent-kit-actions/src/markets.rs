use agent_kit_core::action::{
    ActionContext, ActionDescriptor, ActionError, ActionOutput, Effect, ServiceKind,
};
use serde_json::{Value, json};

use crate::util::{self, ensure_mock, require_object, require_string};

pub(crate) fn quote_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "markets.quote".into(),
        title: "Market quote".into(),
        summary: "Quote a market price for a symbol.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["symbol"],
            "properties": {
                "symbol": { "type": "string" }
            }
        }),
        output_schema: json!({
            "type": "object",
            "required": ["symbol", "price", "currency", "as_of"],
            "properties": {
                "symbol": { "type": "string" },
                "price": { "type": "number" },
                "currency": { "type": "string" },
                "as_of": { "type": "string" }
            }
        }),
        requires: vec![ServiceKind::Markets],
        effect: Effect::Read,
    }
}

pub(crate) async fn quote(ctx: &ActionContext, input: Value) -> Result<ActionOutput, ActionError> {
    require_object(&input)?;
    ensure_mock(ctx, ServiceKind::Markets).await?;
    let symbol = require_string(&input, "symbol")?.to_ascii_uppercase();
    Ok(util::ok(json!({
        "symbol": symbol,
        "price": util::cents_to_number(util::cents_for(&symbol)),
        "currency": "USD",
        "as_of": "2026-01-15T00:00:00Z",
    })))
}

pub(crate) fn candles_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "markets.candles".into(),
        title: "Market candles".into(),
        summary: "Return OHLCV candles for a symbol.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["symbol", "interval"],
            "properties": {
                "symbol": { "type": "string" },
                "interval": { "type": "string", "enum": ["1d", "1h"] },
                "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
            }
        }),
        output_schema: json!({
            "type": "object",
            "required": ["candles"],
            "properties": {
                "candles": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["t", "o", "h", "l", "c", "v"],
                        "properties": {
                            "t": { "type": "integer" },
                            "o": { "type": "number" },
                            "h": { "type": "number" },
                            "l": { "type": "number" },
                            "c": { "type": "number" },
                            "v": { "type": "integer" }
                        }
                    }
                }
            }
        }),
        requires: vec![ServiceKind::Markets],
        effect: Effect::Read,
    }
}

pub(crate) async fn candles(
    ctx: &ActionContext,
    input: Value,
) -> Result<ActionOutput, ActionError> {
    require_object(&input)?;
    ensure_mock(ctx, ServiceKind::Markets).await?;
    let symbol = require_string(&input, "symbol")?.to_ascii_uppercase();
    let interval = require_string(&input, "interval")?;
    if interval != "1d" && interval != "1h" {
        return Err(util::invalid("interval must be 1d or 1h"));
    }
    let limit = util::optional_limit(&input, 10, 100)?;
    let step: i64 = if interval == "1h" { 3600 } else { 86400 };
    let base = i64::from(util::cents_for(&symbol));
    let candles = (0..limit)
        .map(|index| {
            let index = index as i64;
            let open = base + (index % 7) * 15;
            json!({
                "t": 1_767_225_600 + index * step,
                "o": util::cents_to_number(open as u32),
                "h": util::cents_to_number((open + 40) as u32),
                "l": util::cents_to_number((open - 25) as u32),
                "c": util::cents_to_number((open + 10) as u32),
                "v": 1_000 + (util::stable_hash(&format!("{symbol}:{interval}:{index}")) % 5_000),
            })
        })
        .collect::<Vec<_>>();
    Ok(util::ok(json!({ "candles": candles })))
}

pub(crate) fn order_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "markets.order".into(),
        title: "Market order".into(),
        summary: "Refuse live brokerage orders.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["symbol", "side", "quantity"],
            "properties": {
                "symbol": { "type": "string" },
                "side": { "type": "string", "enum": ["buy", "sell"] },
                "quantity": { "type": "number", "exclusiveMinimum": 0 }
            }
        }),
        output_schema: json!({
            "type": "object"
        }),
        requires: vec![ServiceKind::Markets],
        effect: Effect::Mutating,
    }
}

pub(crate) async fn order(
    _ctx: &ActionContext,
    _input: Value,
) -> Result<ActionOutput, ActionError> {
    Err(ActionError::LiveTradingDisabled)
}
