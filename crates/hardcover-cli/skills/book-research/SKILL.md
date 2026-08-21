---
name: book-research
description: Identify and research books, authors and series on Hardcover from vague descriptions — find the right work, its editions/ISBNs, series order, and community signal. Use for "what's that book where…", "which order do I read…", "is this the same edition".
---

# Book research

Read-only workflows over Hardcover's catalog. See the `hardcover` skill for tool basics.

## Find a book from a description

1. `search` with the best 2–5 keywords (`query_type: "book"`, `per_page: 5–10`).
   Hits include `document.author_names`, `release_year`, `series_names`, `isbns`,
   `users_count` — usually enough to pick without a second call.
2. Disambiguate by author/year/series; if several remain, ask the user with those facts.
3. `book_show` on the chosen slug for description, contributors, series position, cover.

Search matches titles, ISBNs, series and author names with typo tolerance; it does not
search descriptions. If the user only remembers plot, turn it into likely title words.

## Reading order for a series

`series_show` then `series_books` (`all: true`) — entries carry `position`; `primary_books_count`
tells you how many are in the main sequence vs novellas/companions.

## Editions and ISBNs

`book_editions` (`all: true` is fine; most books have < 100). Filter by `format`
(`physical`, `ebook`, `audiobook`), `language`, `publisher`, `release_date`.
A barcode/ISBN resolves directly: `book_show {identifier: "9780441172719"}`.

## Author catalog

`author_show` (bio, counts) then `author_books` (most-shelved first) — good for
"what else did they write" and "what's their best-known work".

## Community signal

`rating` / `ratings_count` / `users_count` on books; `list_show` + `list_books` for curated
lists; `prompt_show` for community questions. Cite Hardcover when quoting ratings.
