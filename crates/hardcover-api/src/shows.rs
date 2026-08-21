//! Single-entity lookups and id/slug resolution for non-Book nouns.
use crate::client::Client;
use crate::collections::cover_url;
use crate::error::{Error, Result};
use crate::model::*;
use crate::queries::*;

macro_rules! resolve_by_slug {
    ($self:ident, $ident:expr, $Query:ident, $vars:ident, $field:ident, $what:literal) => {
        match $ident {
            Identifier::Id(id) => Ok(Resolved { id: *id, resolved_by: ResolvedBy::Id }),
            Identifier::Slug(slug) => {
                let data = $self.execute::<$Query>($vars::Variables { slug: slug.clone() }).await?;
                let id = data.$field.first().map(|r| r.id).ok_or_else(|| Error::NotFound(format!("{} with slug {slug}", $what)))?;
                Ok(Resolved { id, resolved_by: ResolvedBy::Slug })
            }
        }
    };
}

impl Client {
    pub async fn resolve_author(&self, ident: &Identifier) -> Result<Resolved> {
        resolve_by_slug!(self, ident, AuthorIdBySlug, author_id_by_slug, authors, "author")
    }
    pub async fn resolve_series(&self, ident: &Identifier) -> Result<Resolved> {
        resolve_by_slug!(self, ident, SeriesIdBySlug, series_id_by_slug, series, "series")
    }
    pub async fn resolve_list(&self, ident: &Identifier) -> Result<Resolved> {
        resolve_by_slug!(self, ident, ListIdBySlug, list_id_by_slug, lists, "list")
    }
    pub async fn resolve_prompt(&self, ident: &Identifier) -> Result<Resolved> {
        resolve_by_slug!(self, ident, PromptIdBySlug, prompt_id_by_slug, prompts, "prompt")
    }

    pub async fn author(&self, id: i64) -> Result<Author> {
        let a = self
            .execute::<AuthorById>(author_by_id::Variables { id })
            .await?
            .authors_by_pk
            .ok_or_else(|| Error::NotFound(format!("author {id}")))?;
        Ok(Author {
            id: a.id,
            slug: a.slug.unwrap_or_default(),
            name: a.name,
            bio: a.bio,
            born_year: a.born_year,
            death_year: a.death_year,
            books_count: a.books_count,
            users_count: a.users_count,
            image_url: cover_url(&a.cached_image),
        })
    }

    pub async fn series(&self, id: i64) -> Result<Series> {
        let s = self
            .execute::<SeriesById>(series_by_id::Variables { id })
            .await?
            .series_by_pk
            .ok_or_else(|| Error::NotFound(format!("series {id}")))?;
        Ok(Series {
            id: s.id,
            slug: s.slug,
            name: s.name,
            description: s.description,
            is_completed: s.is_completed,
            books_count: s.books_count,
            primary_books_count: s.primary_books_count,
            author: s.author.map(|a| Contributor { id: a.id, slug: a.slug.unwrap_or_default(), name: a.name }),
        })
    }

    pub async fn edition(&self, id: i64) -> Result<Edition> {
        let e = self
            .execute::<EditionById>(edition_by_id::Variables { id })
            .await?
            .editions_by_pk
            .ok_or_else(|| Error::NotFound(format!("edition {id}")))?;
        Ok(Edition {
            id: e.id,
            book_id: e.book_id,
            title: e.title.unwrap_or_default(),
            subtitle: e.subtitle,
            isbn_10: e.isbn_10,
            isbn_13: e.isbn_13,
            asin: e.asin,
            format: reading_format(Some(e.reading_format_id)),
            edition_format: e.edition_format,
            pages: e.pages,
            audio_seconds: e.audio_seconds,
            release_date: e.release_date,
            language: e.language.map(|l| l.language),
            publisher: e.publisher.map(|p| Publisher { id: p.id, name: p.name }),
            cover_url: cover_url(&e.cached_image),
        })
    }

    pub async fn list(&self, id: i64) -> Result<List> {
        let l = self
            .execute::<ListById>(list_by_id::Variables { id })
            .await?
            .lists_by_pk
            .ok_or_else(|| Error::NotFound(format!("list {id}")))?;
        Ok(List {
            id: l.id,
            slug: l.slug,
            name: l.name,
            description: l.description,
            books_count: l.books_count,
            followers_count: l.followers_count,
            likes_count: l.likes_count,
            ranked: l.ranked,
            featured: l.featured,
            owner: UserRef { id: l.user.id, username: l.user.username.unwrap_or_default() },
        })
    }

    pub async fn prompt(&self, id: i64) -> Result<Prompt> {
        let p = self
            .execute::<PromptById>(prompt_by_id::Variables { id })
            .await?
            .prompts_by_pk
            .ok_or_else(|| Error::NotFound(format!("prompt {id}")))?;
        Ok(Prompt {
            id: p.id,
            slug: p.slug,
            question: p.question,
            description: p.description,
            answers_count: p.answers_count,
            books_count: p.books_count,
            users_count: p.users_count,
            owner: UserRef { id: p.user.id, username: p.user.username.unwrap_or_default() },
        })
    }

    pub async fn user_by_username(&self, username: &str) -> Result<UserProfile> {
        let u = self
            .execute::<UserByUsername>(user_by_username::Variables { username: username.to_string() })
            .await?
            .users
            .into_iter()
            .next()
            .ok_or_else(|| Error::NotFound(format!("user {username}")))?;
        Ok(UserProfile {
            id: u.id,
            username: u.username.unwrap_or_default(),
            name: u.name,
            bio: u.bio,
            location: u.location,
            flair: u.flair,
            pro: u.pro,
            books_count: u.books_count,
            followers_count: u.followers_count,
            followed_users_count: u.followed_users_count,
            image_url: cover_url(&u.cached_image),
            created_at: u.created_at,
        })
    }
}
