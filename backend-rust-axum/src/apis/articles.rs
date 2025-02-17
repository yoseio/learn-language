use async_trait::async_trait;
use axum::extract::*;
use axum_extra::extract::{CookieJar, Multipart};
use bytes::Bytes;
use http::Method;
use serde::{Deserialize, Serialize};

use crate::{models, types::*};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum CreateArticleResponse {
    /// Single article
    Status201_SingleArticle(models::CreateArticle201Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum DeleteArticleResponse {
    /// No content
    Status200_NoContent,
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetArticleResponse {
    /// Single article
    Status200_SingleArticle(models::CreateArticle201Response),
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetArticlesResponse {
    /// Multiple articles
    Status200_MultipleArticles(models::GetArticlesFeed200Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetArticlesFeedResponse {
    /// Multiple articles
    Status200_MultipleArticles(models::GetArticlesFeed200Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum UpdateArticleResponse {
    /// Single article
    Status200_SingleArticle(models::CreateArticle201Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

/// Articles
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Articles {
    type Claims;

    /// Create an article.
    ///
    /// CreateArticle - POST /api/articles
    async fn create_article(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        body: models::CreateArticleRequest,
    ) -> Result<CreateArticleResponse, ()>;

    /// Delete an article.
    ///
    /// DeleteArticle - DELETE /api/articles/{slug}
    async fn delete_article(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: models::DeleteArticlePathParams,
    ) -> Result<DeleteArticleResponse, ()>;

    /// Get an article.
    ///
    /// GetArticle - GET /api/articles/{slug}
    async fn get_article(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        path_params: models::GetArticlePathParams,
    ) -> Result<GetArticleResponse, ()>;

    /// Get recent articles globally.
    ///
    /// GetArticles - GET /api/articles
    async fn get_articles(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        query_params: models::GetArticlesQueryParams,
    ) -> Result<GetArticlesResponse, ()>;

    /// Get recent articles from users you follow.
    ///
    /// GetArticlesFeed - GET /api/articles/feed
    async fn get_articles_feed(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        query_params: models::GetArticlesFeedQueryParams,
    ) -> Result<GetArticlesFeedResponse, ()>;

    /// Update an article.
    ///
    /// UpdateArticle - PUT /api/articles/{slug}
    async fn update_article(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: models::UpdateArticlePathParams,
        body: models::UpdateArticleRequest,
    ) -> Result<UpdateArticleResponse, ()>;
}
