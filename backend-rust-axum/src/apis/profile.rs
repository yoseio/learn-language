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
pub enum FollowUserByUsernameResponse {
    /// Profile
    Status200_Profile(models::GetProfileByUsername200Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetProfileByUsernameResponse {
    /// Profile
    Status200_Profile(models::GetProfileByUsername200Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum UnfollowUserByUsernameResponse {
    /// Profile
    Status200_Profile(models::GetProfileByUsername200Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

/// Profile
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait Profile {
    type Claims;

    /// Follow a user.
    ///
    /// FollowUserByUsername - POST /api/profiles/{username}/follow
    async fn follow_user_by_username(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: models::FollowUserByUsernamePathParams,
    ) -> Result<FollowUserByUsernameResponse, ()>;

    /// Get a profile.
    ///
    /// GetProfileByUsername - GET /api/profiles/{username}
    async fn get_profile_by_username(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        path_params: models::GetProfileByUsernamePathParams,
    ) -> Result<GetProfileByUsernameResponse, ()>;

    /// Unfollow a user.
    ///
    /// UnfollowUserByUsername - DELETE /api/profiles/{username}/follow
    async fn unfollow_user_by_username(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        path_params: models::UnfollowUserByUsernamePathParams,
    ) -> Result<UnfollowUserByUsernameResponse, ()>;
}
