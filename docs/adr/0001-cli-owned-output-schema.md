# 0001 — The CLI owns its output schema

**Status:** Accepted · **Date:** 2026-08-21

## Context

Hardcover exposes a Hasura-style GraphQL API. The cheapest CLI would forward
GraphQL responses verbatim. The primary consumer of this CLI is an Agent that
parses output programmatically, and Hardcover's API is labelled beta with the
explicit warning that it may change without notice.

## Decision

The CLI defines and versions its own JSON output shape (envelope
`{schema, data, meta}` for `json`; bare objects for `ndjson`), mapped from the
upstream response by the `hardcover-api` crate. The upstream payload is
available only behind `--raw`.

## Consequences

- Agents depend on a shape we control; upstream schema changes are absorbed in
  the mapping layer rather than breaking every consumer.
- We take on the maintenance of domain structs and a mapping for every
  supported operation, and must version the schema deliberately.
- A future MCP server reuses the same domain structs unchanged.
- Power users retain full access to upstream data via `--raw`.

## Alternatives considered

- **Verbatim passthrough** — zero mapping cost, but couples every consumer to a
  beta API and leaks Hasura naming (`user_books`, `cached_contributors`).
- **Passthrough plus light renaming** — still tied to upstream structure;
  halfway house with most of the cost and few of the benefits.
