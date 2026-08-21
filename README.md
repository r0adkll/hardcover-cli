# hardcover-cli

A command-line client for the [Hardcover](https://hardcover.app) book-tracking API, built
**for agents first**: every command emits a stable, versioned JSON shape when piped, and
structured errors with machine-readable codes on stderr. Humans get readable text on a TTY.

```
$ hardcover search "name of the wind" --per-page 3
$ hardcover book show the-name-of-the-wind
$ hardcover book show 978-0-7564-0474-1 --format json | jq .data.contributors
$ hardcover series books kingkiller-chronicle --all --format ndjson
```

## Install

```sh
cargo install --git https://github.com/r0adkll/hardcover-cli hardcover-cli
```

Prebuilt binaries for macOS, Linux and Windows are attached to each
[GitHub release](https://github.com/r0adkll/hardcover-cli/releases) with shell/PowerShell installers.

## Authenticate

Create a personal access token at <https://hardcover.app/account/api>, then:

```sh
hardcover login            # prompts, verifies the token, stores it in the OS keychain
echo "$TOKEN" | hardcover login   # non-interactive
```

Token precedence: `--token` flag → `HARDCOVER_TOKEN` env var → OS keychain.
Every request requires a token; Hardcover has no anonymous access.

## Commands

| Command | What it returns |
|---|---|
| `search <query> [--type book\|author\|series\|character\|list\|prompt\|publisher\|user]` | Full-text hits with stable `id`, `slug`, `label` plus the raw search document |
| `book show <id\|slug\|isbn>` | A work with contributors, series membership, counts and cover URL |
| `book editions <id\|slug\|isbn>` | Its editions (ISBNs, format, publisher, language) |
| `author show <id\|slug>` / `author books <id\|slug>` | Author profile / their books, most-shelved first |
| `series show <id\|slug>` / `series books <id\|slug>` | Series / its books in position order |
| `list show <id\|slug>` / `list books <id\|slug>` | List / its books |
| `edition show <id>` | One edition |
| `prompt show <id\|slug>` | A community prompt |
| `user show <username>` | A public profile |
| `whoami` / `login` / `logout` | Credential management |
| `schema` | Machine-readable description of every command, argument, format and error code |

Identifiers: all-digit → id, ISBN-10/13 (hyphens OK) → ISBN, otherwise slug.
Force a form with `id:`, `slug:` or `isbn:` prefixes. Output always carries both `id` and `slug`,
and `meta.resolved_by` says which form matched.

## Output

`--format auto|json|ndjson|table|plain` (default `auto`: JSON when stdout is not a terminal).

```jsonc
{ "schema": "hardcover-cli/1", "data": { ... }, "meta": { "resolved_by": "slug" } }
```

Collections accept `--limit`, `--offset`, and `--all` (pages until exhausted or `--max-rows`,
default 1000; `meta.truncated` is `true` if the cap was hit). `ndjson` streams one bare object
per line and is the natural partner of `--all`. `--raw` prints the upstream GraphQL payload instead.

Errors go to stderr as `{"error": {"code": "...", "message": "..."}}`:

| code | exit | meaning |
|---|---|---|
| `usage_error` | 2 | bad arguments |
| `auth_required`, `invalid_token`, `insufficient_scope` | 3 | credential problems |
| `not_found` | 4 | nothing matched the identifier |
| `rate_limited` | 5 | upstream limit; includes `retry_after_secs` |
| `network_error`, `upstream_error` | 6 | transport or API failure |

Rate-limited requests are retried with backoff (honouring `Retry-After`) up to 3 attempts;
`--no-retry` disables that.

## Configuration

`~/.config/hardcover/config.toml` (macOS: `~/Library/Application Support/hardcover/config.toml`;
override with `HARDCOVER_CONFIG_DIR`):

```toml
format = "json"     # default --format
username = "you"    # written by `login`
```

The token itself is only ever stored in the OS keychain.

## Hardcover's terms

Using this tool means agreeing to Hardcover's
[API terms](https://docs.hardcover.app/api/getting-started/): backend/personal use only,
no training of public or commercial models on the data, and user-owned data (libraries,
reviews, ratings) only on behalf of a consenting user. This CLI currently exposes only
**content data** (books, editions, authors, series, lists, prompts, public profiles);
reading and writing your own library is a planned milestone.

## Development

```sh
cargo test                         # fixtures recorded from the real API; no network needed
HARDCOVER_TOKEN=... cargo xtask introspect   # refresh crates/hardcover-api/schema.json
```

Layout: `crates/hardcover-api` is a typed client you can depend on directly (the CLI and a
future MCP server are thin layers over it). Domain vocabulary lives in [CONTEXT.md](CONTEXT.md);
decisions in [docs/adr](docs/adr). Agent-oriented usage notes are in [AGENTS.md](AGENTS.md).

## License

MIT or Apache-2.0, at your option.
