# Using hardcover-cli as an agent

If you're reading this inside an agent host, the user can run `hardcover agent setup <host>`
to give you the MCP tools and three skills (`hardcover`, `reading-log`, `book-research`).

Two ways in: as **MCP tools** (`hardcover mcp serve` — tool names match the commands
below with underscores, e.g. `book_show`, `library_set_status`; same JSON, same error
codes, `dry_run` on writes) or by **running the CLI**. The rest of this file is written
for the CLI; everything maps 1:1.

Start with `hardcover schema --format json` — it lists every command, argument, output
format and error code, generated from the same source as `--help`.

## Rules of thumb

- **Always pass `--format json`** (or `ndjson` for collections). Don't rely on TTY detection.
- Parse `data`; read `meta` for paging (`limit`, `offset`, `count`, `truncated`) and
  identifier resolution (`resolved_by`).
- On non-zero exit, parse **stderr** as JSON and branch on `error.code`:
  - `auth_required` / `invalid_token` → the human must run `hardcover login`.
  - `not_found` → try `hardcover search` to find the right identifier.
  - `rate_limited` → wait `error.retry_after_secs` seconds (retries are already built in).
- Prefer slugs or ids from a previous result over free-text; `search` is the only fuzzy step.
- `search` hits include `document`, the raw search record, with fields like `author_names`,
  `isbns`, `series_names`, `release_year` — usually enough to avoid a follow-up `show`.
- Hardcover's free tier allows 60 requests/minute and 5,000/day. `--all` can burn many
  requests; set `--limit` high (≤100 is reasonable) and `--max-rows` to what you need.

## Typical flows

```sh
# Find a book and its series
hardcover search "mistborn" --per-page 5 --format json
hardcover book show mistborn-the-final-empire --format json
hardcover series books mistborn --all --format ndjson

# Resolve an ISBN from a scanned barcode
hardcover book show 9780765311788 --format json

# Who is this author, what have they written
hardcover author show brandon-sanderson --format json
hardcover author books brandon-sanderson --limit 50 --format ndjson


# What is the human currently reading?
hardcover library list --status currently_reading --format json
hardcover library show iron-gold --format json      # reads, progress %, rating, review
```

`library` is always the authenticated user's own data; there is no way to read someone
else's library through this tool. `status` values: `want_to_read`, `currently_reading`,
`read`, `paused`, `did_not_finish`, `ignored` (`--status` also accepts `reading`, `dnf`, `want`).

## Writing to the library

Only do this when the human has asked for it. Each write returns `before`/`after`:

```sh
hardcover library set-status mistborn-the-final-empire currently_reading --format json
hardcover library progress mistborn-the-final-empire --pages 120 --format json
hardcover library progress mistborn-the-final-empire --finished today --format json
hardcover library rate mistborn-the-final-empire 4.5 --format json
hardcover library remove mistborn-the-final-empire --dry-run --format json   # preview first
```

- Add `--dry-run` to preview: `data.dry_run` is `true` and `data.planned` shows the intent.
- Check `data.after` rather than assuming success; Hardcover applies side effects
  (e.g. rating → status becomes `read`).
- `remove` is the only destructive command; prefer `set-status … ignored` if the human
  just wants a book hidden.

## Not available yet

Reviews, lists, journals, and other users' data. `prompt books` is not exposed to API tokens upstream.
