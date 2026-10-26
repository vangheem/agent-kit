use agent_kit_core::action::{
    ActionContext, ActionDescriptor, ActionError, ActionOutput, Effect, ServiceKind,
};
use serde_json::{Value, json};

use crate::util::{self, ensure_mock, reject_card_numbers, require_object, require_string};

pub(crate) fn search_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "shop.search".into(),
        title: "Shop search".into(),
        summary: "Search shop products by query.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 20 }
            }
        }),
        output_schema: json!({
            "type": "object",
            "required": ["products"],
            "properties": {
                "products": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["id", "title", "price", "url", "merchant"],
                        "properties": {
                            "id": { "type": "string" },
                            "title": { "type": "string" },
                            "price": {
                                "type": "object",
                                "required": ["amount", "currency"],
                                "properties": {
                                    "amount": { "type": "number" },
                                    "currency": { "type": "string" }
                                }
                            },
                            "url": { "type": "string" },
                            "merchant": { "type": "string" }
                        }
                    }
                }
            }
        }),
        requires: vec![ServiceKind::Shop],
        effect: Effect::Read,
    }
}

pub(crate) async fn search(ctx: &ActionContext, input: Value) -> Result<ActionOutput, ActionError> {
    require_object(&input)?;
    ensure_mock(ctx, ServiceKind::Shop).await?;
    let query = require_string(&input, "query")?;
    let limit = util::optional_limit(&input, 5, 20)?;
    let merchants = ["Mock Market", "Example Goods", "Kit Supply"];
    let products = (1..=limit)
        .map(|index| {
            let id = format!(
                "prod_{:016x}",
                util::stable_hash(&format!("{query}:{index}"))
            );
            json!({
                "id": id,
                "title": format!("{query} item {index}"),
                "price": util::money(util::cents_for(&id)),
                "url": format!("https://example.test/products/{id}"),
                "merchant": merchants[(index - 1) % merchants.len()],
            })
        })
        .collect::<Vec<_>>();
    Ok(util::ok(json!({ "products": products })))
}

pub(crate) fn quote_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "shop.quote".into(),
        title: "Shop quote".into(),
        summary: "Quote price and shipping for a shop product.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["product_id"],
            "properties": {
                "product_id": { "type": "string" }
            }
        }),
        output_schema: json!({
            "type": "object",
            "required": ["product_id", "available", "price", "shipping_estimate"],
            "properties": {
                "product_id": { "type": "string" },
                "available": { "type": "boolean" },
                "price": {
                    "type": "object",
                    "required": ["amount", "currency"],
                    "properties": {
                        "amount": { "type": "number" },
                        "currency": { "type": "string" }
                    }
                },
                "shipping_estimate": { "type": "string" }
            }
        }),
        requires: vec![ServiceKind::Shop],
        effect: Effect::Read,
    }
}

pub(crate) async fn quote(ctx: &ActionContext, input: Value) -> Result<ActionOutput, ActionError> {
    require_object(&input)?;
    ensure_mock(ctx, ServiceKind::Shop).await?;
    let product_id = require_string(&input, "product_id")?;
    Ok(util::ok(json!({
        "product_id": product_id,
        "available": true,
        "price": util::money(util::cents_for(&product_id)),
        "shipping_estimate": "3-5 business days",
    })))
}

pub(crate) fn checkout_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "shop.checkout".into(),
        title: "Shop checkout".into(),
        summary: "Place a mock hold on a shop product. Does not charge a card.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["product_id", "quantity"],
            "properties": {
                "product_id": { "type": "string" },
                "quantity": { "type": "integer", "minimum": 1 }
            }
        }),
        output_schema: json!({
            "type": "object",
            "required": ["status", "hold_id", "product_id", "quantity"],
            "properties": {
                "status": { "type": "string" },
                "hold_id": { "type": "string" },
                "product_id": { "type": "string" },
                "quantity": { "type": "integer" }
            }
        }),
        requires: vec![ServiceKind::Shop],
        effect: Effect::Mutating,
    }
}

pub(crate) async fn checkout(
    ctx: &ActionContext,
    input: Value,
) -> Result<ActionOutput, ActionError> {
    require_object(&input)?;
    let product_id = require_string(&input, "product_id")?;
    let quantity = util::require_u32(&input, "quantity", 1, 10_000)?;
    reject_card_numbers(&input)?;
    ensure_mock(ctx, ServiceKind::Shop).await?;
    Ok(util::ok(json!({
        "status": "mock_hold",
        "hold_id": util::hold_id("shop", &format!("{product_id}:{quantity}")),
        "product_id": product_id,
        "quantity": quantity,
    })))
}
