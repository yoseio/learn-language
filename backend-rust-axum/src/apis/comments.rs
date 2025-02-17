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
pub enum CreateArticleCommentResponse {
    /// Single comment
    Status200_SingleComment(models::CreateArticleComment200Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum DeleteArticleCommentResponse {
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
pub enum GetArticleCommentsResponse {
    /// Multiple comments
    Status200_MultipleComments(models::GetArticleComments200Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

/// Comments
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Comments {
    type Claims;

    /// Create a comment for an article.
    ///
    /// CreateArticleComment - POST /api/articles/{slug}/comments
    async fn create_article_comment(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: models::CreateArticleCommentPathParams,
        body: models::CreateArticleCommentRequest,
    ) -> Result<CreateArticleCommentResponse, ()>;

    /// Delete a comment for an article.
    ///
    /// DeleteArticleComment - DELETE /api/articles/{slug}/comments/{id}
    async fn delete_article_comment(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: models::DeleteArticleCommentPathParams,
    ) -> Result<DeleteArticleCommentResponse, ()>;

    /// Get comments for an article.
    ///
    /// GetArticleComments - GET /api/articles/{slug}/comments
    async fn get_article_comments(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        path_params: models::GetArticleCommentsPathParams,
    ) -> Result<GetArticleCommentsResponse, ()>;
}
