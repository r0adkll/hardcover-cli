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
#[graphql(schema_path = "schema.json", query_path = "queries/book_id_by_slug.graphql", response_derives = "Debug, Clone")]
pub struct BookIdBySlug;

#[derive(GraphQLQuery)]
#[graphql(schema_path = "schema.json", query_path = "queries/book_id_by_isbn.graphql", response_derives = "Debug, Clone")]
pub struct BookIdByIsbn;

#[derive(GraphQLQuery)]
#[graphql(schema_path = "schema.json", query_path = "queries/author_books.graphql", response_derives = "Debug, Clone")]
pub struct AuthorBooks;

#[derive(GraphQLQuery)]
#[graphql(schema_path = "schema.json", query_path = "queries/series_books.graphql", response_derives = "Debug, Clone")]
pub struct SeriesBooks;

#[derive(GraphQLQuery)]
#[graphql(schema_path = "schema.json", query_path = "queries/list_books.graphql", response_derives = "Debug, Clone")]
pub struct ListBooks;

#[derive(GraphQLQuery)]
#[graphql(schema_path = "schema.json", query_path = "queries/book_editions.graphql", response_derives = "Debug, Clone")]
pub struct BookEditions;
