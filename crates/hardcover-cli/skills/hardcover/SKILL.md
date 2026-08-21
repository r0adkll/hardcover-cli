---
name: hardcover
description: How to use the Hardcover.app tools (MCP server or `hardcover` CLI) for book data and the user's reading library — identifiers, output shape, errors, paging, safe writes.
---

# Hardcover tools

Hardcover is a book-tracking service. The `hardcover` MCP server (and the `hardcover` CLI)
exposes its catalog and the authenticated user's own library.

## Two ways in

- **MCP tools** (preferred when available): `search`, `book_show`, `book_editions`,
  `author_show`, `author_books`, `series_show`, `series_books`, `list_show`, `list_books`,
  `edition_show`, `prompt_show`, `user_show`, `whoami`, `library_list`, `library_show`,
  `library_set_status`, `library_rate`, `library_progress`, `library_remove`.
- **CLI**: same names with spaces (`hardcover book show …`). Always add `--format json`.
  `hardcover schema --format json` describes every command and error code.

## Identifiers

Every entity accepts a numeric `id` or a `slug`; books also accept ISBN-10/13 (hyphens OK).
Prefix `id:`, `slug:` or `isbn:` to force a form. Results always carry both `id` and `slug`;
`meta.resolved_by` says which form matched. Prefer ids/slugs from a previous result over
free text — `search` is the only fuzzy step.

## Output shape

`{ "schema": "hardcover-cli/1", "data": …, "meta": … }`. Read `data`; use `meta` for paging
(`limit`, `offset`, `count`, `truncated`) and resolution (`resolved_by`).

## Errors

Failures come back as `{ "error": { "code", "message", … } }` (tool error / stderr + exit code):

| code | meaning | do |
|---|---|---|
| `auth_required`, `invalid_token` | no usable token | ask the human to run `hardcover login` |
| `not_found` | nothing matched | try `search`, or check the identifier form |
| `rate_limited` | upstream limit | wait `retry_after_secs`; retries are already built in |
| `usage_error` | bad argument | fix the call |
| `upstream_error`, `network_error` | API/transport | report; retry once later |

## Paging

Collections take `limit`/`offset`; `all: true` pages until exhausted or `max_rows`
(default 1000, 0 = unlimited) and sets `meta.truncated`. Free accounts get 60 requests/min,
5,000/day — keep `limit` high (≤100) rather than many small pages.

## Library (user data)

`library_*` tools act only on the token owner's own library; there is no way to read or
change anyone else's. Reading status values: `want_to_read`, `currently_reading`, `read`,
`paused`, `did_not_finish`, `ignored`.

## Writes

`library_set_status`, `library_rate`, `library_progress`, `library_remove` change real data.
- Only write when the human asked for it; preview with `dry_run: true` when unsure.
- Every write returns `{action, before, after}` — check `after`, don't assume.
- Hardcover side effects: rating a book marks it `read`; progress updates the open read
  (the one without a finish date) or starts one.
- `library_remove` deletes the entry, reads, rating and review. Prefer
  `library_set_status … ignored` to hide a book.
