# blob

Read `docs/PLAN.md` and `docs/CONTRACTS.md`. Implement `crates/agent-kit-blob` only.

`connect` returns the `pg` or `fs` `BlobStore`. Metadata writes go through SQL against the pool using the `blobs` and `blob_payloads` tables from the contract. You may call `agent_kit_db::Db` helpers if they already match; otherwise use the pool directly so you are not blocked. Do not add a third backend.

`fs` stores bytes at `{root}/{uuid}`. Clean up the file if the metadata insert fails. Reject payloads over 10 MiB.

Add a unit test for the size limit or for filesystem path joining. Do not start compose. Do not edit the migration; db owns it.

Package name `agent-kit-blob`. Path deps on `../agent-kit-core` and `../agent-kit-db` only if you call `Db`. Do not edit the workspace `Cargo.toml`.

Do not commit.
