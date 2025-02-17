pub mod articles;
pub mod comments;
pub mod favorites;
pub mod profile;
pub mod tags;
pub mod user_and_authentication;

/// API Key Authentication - Header.
#[async_trait::async_trait]
pub trait ApiKeyAuthHeader {
    type Claims;

    /// Extracting Claims from Header. Return None if the Claims is invalid.
    async fn extract_claims_from_header(
        &self,
        headers: &axum::http::header::HeaderMap,
        key: &str,
    ) -> Option<Self::Claims>;
}
