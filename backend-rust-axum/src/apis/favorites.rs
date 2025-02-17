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
pub enum CreateArticleFavoriteResponse {
    /// Single article
    Status200_SingleArticle(models::CreateArticle201Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum DeleteArticleFavoriteResponse {
    /// Single article
    Status200_SingleArticle(models::CreateArticle201Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

/// Favorites
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Favorites {
    type Claims;

    /// Favorite an article.
    ///
    /// CreateArticleFavorite - POST /api/articles/{slug}/favorite
    async fn create_article_favorite(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: models::CreateArticleFavoritePathParams,
    ) -> Result<CreateArticleFavoriteResponse, ()>;

    /// Unfavorite an article.
    ///
    /// DeleteArticleFavorite - DELETE /api/articles/{slug}/favorite
    async fn delete_article_favorite(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: models::DeleteArticleFavoritePathParams,
    ) -> Result<DeleteArticleFavoriteResponse, ()>;
}
