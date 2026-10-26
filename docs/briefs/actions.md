# actions

Read `docs/PLAN.md` and `docs/CONTRACTS.md`. Implement `crates/agent-kit-actions` only.

`register()` returns the eleven actions in the contract table. Mock results are deterministic. No real shopping, flight, payment, or brokerage HTTP calls. Unknown providers return `provider_not_configured`.

`web.fetch` may use `reqwest` with rustls. Enforce the SSRF rules and the 2 MiB / 10 second caps. Store the raw body with `ctx.blobs` and set `output_blob_id`. If blob storage fails, return `internal` and do not pretend the fetch was stored.

`markets.order` returns `live_trading_disabled`. Checkout and book return `mock_hold` only when the mock provider is selected. They do not accept card numbers.

Add one test that mock `web.search` is deterministic and one test that a private URL is rejected before any request. Do not start compose.

Package name `agent-kit-actions`. Path dep on `../agent-kit-core`. Do not edit the workspace `Cargo.toml`. Do not commit.
