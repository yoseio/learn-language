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
pub enum CreateUserResponse {
    /// User
    Status201_User(models::Login200Response),
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum GetCurrentUserResponse {
    /// User
    Status200_User(models::Login200Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum LoginResponse {
    /// User
    Status200_User(models::Login200Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
#[allow(clippy::large_enum_variant)]
pub enum UpdateCurrentUserResponse {
    /// User
    Status200_User(models::Login200Response),
    /// Unauthorized
    Status401_Unauthorized,
    /// Unexpected error
    Status422_UnexpectedError(models::GenericErrorModel),
}

/// UserAndAuthentication
#[async_trait]
#[allow(clippy::ptr_arg)]
pub trait UserAndAuthentication {
    type Claims;

    /// CreateUser - POST /api/users
    async fn create_user(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        body: models::CreateUserRequest,
    ) -> Result<CreateUserResponse, ()>;

    /// Get current user.
    ///
    /// GetCurrentUser - GET /api/user
    async fn get_current_user(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
    ) -> Result<GetCurrentUserResponse, ()>;

    /// Existing user login.
    ///
    /// Login - POST /api/users/login
    async fn login(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        body: models::LoginRequest,
    ) -> Result<LoginResponse, ()>;

    /// Update current user.
    ///
    /// UpdateCurrentUser - PUT /api/user
    async fn update_current_user(
        &self,
        method: Method,
        host: Host,
        cookies: CookieJar,
        claims: Self::Claims,
        body: models::UpdateCurrentUserRequest,
    ) -> Result<UpdateCurrentUserResponse, ()>;
}
