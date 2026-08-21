---
name: reading-log
description: Keep the user's Hardcover reading log current — answer "what am I reading", record progress, mark books finished, rate them. Use when the user talks about their current reads, pages, finishing or rating a book.
---

# Reading log

Workflows over the user's own Hardcover library. Requires the `hardcover` tools (see the
`hardcover` skill for identifiers, errors and the write contract).

## What am I reading?

`library_list` with `status: "currently_reading"`. Present title, author (from `book`),
and the open read's progress if the user wants detail (`library_show` per book gives
`reads[]` with `progress`, `progress_pages`, `started_at`).

## Record progress

"I'm on page 120 of Dune" → `library_progress` `{identifier: "dune", pages: 120}`.
- For audiobooks use `seconds`.
- If the book isn't shelved yet the tool adds it as `currently_reading` and starts a read.
- Confirm the match if the title was ambiguous (use `search` first, then the slug).

## Finished a book

`library_progress` `{identifier, finished: "today"}` closes the open read;
then, if the user gives a rating, `library_rate` `{identifier, rating}` (0.5–5 in half stars).
Rating alone also marks the book `read`.

## Review

"Here's my review of X: …" → `library_review` `{identifier, text, spoilers?}` (Markdown).
The book must already be shelved (shelve it as `read` first if not). Replaces any existing
review; there is no clear — tell the user to edit on hardcover.app if they want it gone.
Read it back with `library_show` (`review` is the Markdown, `review_html` the rendered form).

## Start / shelve

- "Add X to my to-read" → `library_set_status` `{identifier, status: "want_to_read"}`.
- "Start X" → `library_set_status … "currently_reading"` (a read starts on first progress).
- "Gave up on X" → `status: "did_not_finish"`; "pausing X" → `"paused"`.

## Norms

- State what you're about to change and the book you resolved before writing, unless the
  user's instruction was unambiguous.
- After a write, report from `after` (status, rating, progress), not from your intent.
- Never touch `library_remove` unless the user explicitly asks to delete the entry.
