use agent_kit_core::action::{
    ActionContext, ActionDescriptor, ActionError, ActionOutput, Effect, ServiceKind,
};
use serde_json::{Value, json};

use crate::util::{self, ensure_mock, reject_card_numbers, require_object, require_string};

pub(crate) fn search_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "flights.search".into(),
        title: "Flight search".into(),
        summary: "Search flight offers by origin, destination, and date.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["origin", "destination", "depart_date"],
            "properties": {
                "origin": { "type": "string" },
                "destination": { "type": "string" },
                "depart_date": { "type": "string" },
                "return_date": { "type": "string" },
                "adults": { "type": "integer", "minimum": 1 }
            }
        }),
        output_schema: json!({
            "type": "object",
            "required": ["offers"],
            "properties": {
                "offers": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["id", "carrier", "depart_at", "arrive_at", "stops", "price"],
                        "properties": {
                            "id": { "type": "string" },
                            "carrier": { "type": "string" },
                            "depart_at": { "type": "string" },
                            "arrive_at": { "type": "string" },
                            "stops": { "type": "integer" },
                            "price": {
                                "type": "object",
                                "required": ["amount", "currency"],
                                "properties": {
                                    "amount": { "type": "number" },
                                    "currency": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            }
        }),
        requires: vec![ServiceKind::Flights],
        effect: Effect::Read,
    }
}

pub(crate) async fn search(ctx: &ActionContext, input: Value) -> Result<ActionOutput, ActionError> {
    require_object(&input)?;
    ensure_mock(ctx, ServiceKind::Flights).await?;
    let origin = require_string(&input, "origin")?.to_ascii_uppercase();
    let destination = require_string(&input, "destination")?.to_ascii_uppercase();
    let depart_date = util::require_date(&input, "depart_date")?;
    let return_date = util::optional_date(&input, "return_date")?;
    let adults = util::optional_u32(&input, "adults", 1, 1, 9)?;
    let carriers = ["Mock Air", "Example Airways", "Kit Jet"];
    let depart_hours = [8, 11, 16];
    let offers = (0..3)
        .map(|index| {
            let id = format!(
                "offer_{:016x}",
                util::stable_hash(&format!(
                    "{origin}|{destination}|{depart_date}|{}|{adults}|{index}",
                    return_date.as_deref().unwrap_or("-")
                ))
            );
            json!({
                "id": id,
                "carrier": carriers[index],
                "depart_at": format!("{depart_date}T{:02}:00:00Z", depart_hours[index]),
                "arrive_at": format!("{depart_date}T{:02}:30:00Z", depart_hours[index] + 4),
                "stops": index,
                "price": util::money(util::cents_for(&id)),
            })
        })
        .collect::<Vec<_>>();
    Ok(util::ok(json!({ "offers": offers })))
}

pub(crate) fn quote_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "flights.quote".into(),
        title: "Flight quote".into(),
        summary: "Quote a flight offer.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["offer_id"],
            "properties": {
                "offer_id": { "type": "string" }
            }
        }),
        output_schema: json!({
            "type": "object",
            "required": ["offer_id", "price", "expires_at"],
            "properties": {
                "offer_id": { "type": "string" },
                "price": {
                    "type": "object",
                    "required": ["amount", "currency"],
                    "properties": {
                        "amount": { "type": "number" },
                        "currency": { "type": "string" }
                    }
                },
                "expires_at": { "type": "string" }
            }
        }),
        requires: vec![ServiceKind::Flights],
        effect: Effect::Read,
    }
}

pub(crate) async fn quote(ctx: &ActionContext, input: Value) -> Result<ActionOutput, ActionError> {
    require_object(&input)?;
    ensure_mock(ctx, ServiceKind::Flights).await?;
    let offer_id = require_string(&input, "offer_id")?;
    let day = (util::stable_hash(&offer_id) % 28) + 1;
    Ok(util::ok(json!({
        "offer_id": offer_id,
        "price": util::money(util::cents_for(&offer_id)),
        "expires_at": format!("2027-06-{day:02}T00:00:00Z"),
    })))
}

pub(crate) fn book_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "flights.book".into(),
        title: "Flight hold".into(),
        summary: "Place a mock hold on a flight offer. Does not ticket or charge a card.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["offer_id"],
            "properties": {
                "offer_id": { "type": "string" }
            }
        }),
        output_schema: json!({
            "type": "object",
            "required": ["status", "hold_id", "offer_id"],
            "properties": {
                "status": { "type": "string" },
                "hold_id": { "type": "string" },
                "offer_id": { "type": "string" }
            }
        }),
        requires: vec![ServiceKind::Flights],
        effect: Effect::Mutating,
    }
}

pub(crate) async fn book(ctx: &ActionContext, input: Value) -> Result<ActionOutput, ActionError> {
    require_object(&input)?;
    let offer_id = require_string(&input, "offer_id")?;
    reject_card_numbers(&input)?;
    ensure_mock(ctx, ServiceKind::Flights).await?;
    Ok(util::ok(json!({
        "status": "mock_hold",
        "hold_id": util::hold_id("flights", &offer_id),
        "offer_id": offer_id,
    })))
}
