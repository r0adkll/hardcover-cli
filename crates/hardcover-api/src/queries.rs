use crate::scalars::*;
use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/book.graphql",
    response_derives = "Debug, Clone"
)]
pub struct BookById;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/me.graphql",
    response_derives = "Debug, Clone"
)]
pub struct Me;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/search.graphql",
    response_derives = "Debug, Clone"
)]
pub struct Search;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/book_id_by_slug.graphql",
    response_derives = "Debug, Clone"
)]
pub struct BookIdBySlug;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/book_id_by_isbn.graphql",
    response_derives = "Debug, Clone"
)]
pub struct BookIdByIsbn;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/author_books.graphql",
    response_derives = "Debug, Clone"
)]
pub struct AuthorBooks;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/series_books.graphql",
    response_derives = "Debug, Clone"
)]
pub struct SeriesBooks;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/list_books.graphql",
    response_derives = "Debug, Clone"
)]
pub struct ListBooks;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/book_editions.graphql",
    response_derives = "Debug, Clone"
)]
pub struct BookEditions;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/author_by_id.graphql",
    response_derives = "Debug, Clone"
)]
pub struct AuthorById;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/series_by_id.graphql",
    response_derives = "Debug, Clone"
)]
pub struct SeriesById;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/edition_by_id.graphql",
    response_derives = "Debug, Clone"
)]
pub struct EditionById;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/list_by_id.graphql",
    response_derives = "Debug, Clone"
)]
pub struct ListById;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/prompt_by_id.graphql",
    response_derives = "Debug, Clone"
)]
pub struct PromptById;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/user_by_username.graphql",
    response_derives = "Debug, Clone"
)]
pub struct UserByUsername;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/author_id_by_slug.graphql",
    response_derives = "Debug, Clone"
)]
pub struct AuthorIdBySlug;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/series_id_by_slug.graphql",
    response_derives = "Debug, Clone"
)]
pub struct SeriesIdBySlug;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/list_id_by_slug.graphql",
    response_derives = "Debug, Clone"
)]
pub struct ListIdBySlug;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/prompt_id_by_slug.graphql",
    response_derives = "Debug, Clone"
)]
pub struct PromptIdBySlug;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/library.graphql",
    response_derives = "Debug, Clone"
)]
pub struct Library;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/library_entry.graphql",
    response_derives = "Debug, Clone"
)]
pub struct LibraryEntryQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/library_writes.graphql",
    response_derives = "Debug, Clone"
)]
pub struct InsertUserBook;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/library_writes.graphql",
    response_derives = "Debug, Clone"
)]
pub struct UpdateUserBookStatus;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/library_writes.graphql",
    response_derives = "Debug, Clone"
)]
pub struct UpdateUserBookRating;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/library_writes.graphql",
    response_derives = "Debug, Clone"
)]
pub struct InsertUserBookRead;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/library_writes.graphql",
    response_derives = "Debug, Clone"
)]
pub struct UpdateUserBookRead;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/library_writes.graphql",
    response_derives = "Debug, Clone"
)]
pub struct DeleteUserBook;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.json",
    query_path = "queries/library_writes.graphql",
    response_derives = "Debug, Clone"
)]
pub struct UpdateUserBookReview;
