# hardcover-cli — Domain Context

A command-line client for the Hardcover.app API. Its primary consumer is an
**Agent**; humans are a second audience.

## Glossary

**Agent** — An automated system (LLM-driven or scripted) that invokes the CLI
and parses its output. The CLI is designed for Agents first: output must be
structured and predictable before it is pretty.

**Human** — A person running the CLI interactively in a terminal. Secondary
consumer.

**Content Data** — Hardcover data that describes the world of books rather
than any particular user: Books, Editions, Authors, Series, etc. Reading
Content Data is the scope of the first milestone.

**User Data** — Data owned by the authenticated user: reading status, reviews,
ratings, lists, progress. Reading and changing one's own Library, including Reviews, is in scope;
lists and journals are not yet. The CLI never exposes or changes another user's User Data.

**Raw Output** — The upstream Hardcover API payload, passed through untouched.
Opt-in; the default output is the CLI's own stable shape.

## Hardcover domain terms

**Book** — A work, independent of any particular printing. The unit of
ratings, reviews, series membership and reading status.

**Edition** — A specific published form of a Book (ISBN/ASIN, publisher,
language, format: physical, ebook, audio). A Book has many Editions and a
default one per format.

**Contributor** — A person credited on a Book or Edition (author, narrator,
translator, illustrator…). "Author" is the common case, not a separate type.

**Series** — An ordered grouping of Books with a position per Book.

**Tag** — A user-sourced label on a Book. Genres, moods and content warnings
are categories of Tag.

**List** — A user-curated, optionally ranked collection of Books.

**Prompt** — A community question ("best heist novel?") answered with Books.

**Library** — The set of a user's Book entries (User Data). Each entry holds a
Reading Status, optional Rating and Review, and zero or more Reads.

**Reading Status** — One of: Want to Read, Currently Reading, Read, Paused,
Did Not Finish, Ignored. Named in output as `want_to_read`, `currently_reading`,
`read`, `paused`, `did_not_finish`, `ignored`.

**Privacy** — Visibility of a Library entry: Public, Followers, Private.

**Review** — The user's written opinion of a Book, attached to its Library entry. Authored
as Markdown; Hardcover also keeps a rendered HTML form. Optionally flagged as containing
spoilers.

**Read** — One pass through a Book: start date, finish date, progress. A Read with no
finish date is the **open read**; progress updates apply to it.

**Dry Run** — A write command executed without sending the mutation; reports what it
would have done.

**Search** — Full-text lookup across one entity type at a time (book, author,
series, character, list, prompt, publisher, user). Distinct from filtering a
listing.

**Identifier** — Any of a numeric id, a slug, or (for Books/Editions) an
ISBN. Commands accept any form and output always carries both id and slug.

**Token** — The personal API credential issued by Hardcover to an account.
Stored in the operating-system keychain after `login`.
