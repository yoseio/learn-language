use std::collections::HashMap;

use axum::{body::Body, extract::*, response::Response, routing::*};
use axum_extra::extract::{CookieJar, Multipart};
use bytes::Bytes;
use http::{header::CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use tracing::error;
use validator::{Validate, ValidationErrors};

use crate::{header, types::*};

#[allow(unused_imports)]
use crate::{apis, models};

/// Setup API Server.
pub fn new<I, A, C>(api_impl: I) -> Router
where
    I: AsRef<A> + Clone + Send + Sync + 'static,
    A: apis::articles::Articles<Claims = C>
        + apis::comments::Comments<Claims = C>
        + apis::favorites::Favorites<Claims = C>
        + apis::profile::Profile<Claims = C>
        + apis::tags::Tags
        + apis::user_and_authentication::UserAndAuthentication<Claims = C>
        + apis::ApiKeyAuthHeader<Claims = C>
        + 'static,
    C: Send + Sync + 'static,
{
    // build our application with a route
    Router::new()
        .route(
            "/api/articles",
            get(get_articles::<I, A, C>).post(create_article::<I, A, C>),
        )
        .route(
            "/api/articles/:slug",
            delete(delete_article::<I, A, C>)
                .get(get_article::<I, A, C>)
                .put(update_article::<I, A, C>),
        )
        .route(
            "/api/articles/:slug/comments",
            get(get_article_comments::<I, A, C>).post(create_article_comment::<I, A, C>),
        )
        .route(
            "/api/articles/:slug/comments/:id",
            delete(delete_article_comment::<I, A, C>),
        )
        .route(
            "/api/articles/:slug/favorite",
            delete(delete_article_favorite::<I, A, C>).post(create_article_favorite::<I, A, C>),
        )
        .route("/api/articles/feed", get(get_articles_feed::<I, A, C>))
        .route(
            "/api/profiles/:username",
            get(get_profile_by_username::<I, A, C>),
        )
        .route(
            "/api/profiles/:username/follow",
            delete(unfollow_user_by_username::<I, A, C>).post(follow_user_by_username::<I, A, C>),
        )
        .route("/api/tags", get(get_tags::<I, A>))
        .route(
            "/api/user",
            get(get_current_user::<I, A, C>).put(update_current_user::<I, A, C>),
        )
        .route("/api/users", post(create_user::<I, A, C>))
        .route("/api/users/login", post(login::<I, A, C>))
        .with_state(api_impl)
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct CreateArticleBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::CreateArticleRequest,
}

#[tracing::instrument(skip_all)]
fn create_article_validation(
    body: models::CreateArticleRequest,
) -> std::result::Result<(models::CreateArticleRequest,), ValidationErrors> {
    let b = CreateArticleBodyValidator { body: &body };
    b.validate()?;

    Ok((body,))
}
/// CreateArticle - POST /api/articles
#[tracing::instrument(skip_all)]
async fn create_article<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
    Json(body): Json<models::CreateArticleRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::articles::Articles<Claims = C> + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || create_article_validation(body))
        .await
        .unwrap();

    let Ok((body,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .create_article(method, host, cookies, claims, body)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::articles::CreateArticleResponse::Status201_SingleArticle(body) => {
                let mut response = response.status(201);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::articles::CreateArticleResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::articles::CreateArticleResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn delete_article_validation(
    path_params: models::DeleteArticlePathParams,
) -> std::result::Result<(models::DeleteArticlePathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// DeleteArticle - DELETE /api/articles/{slug}
#[tracing::instrument(skip_all)]
async fn delete_article<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::DeleteArticlePathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::articles::Articles<Claims = C> + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || delete_article_validation(path_params))
        .await
        .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .delete_article(method, host, cookies, claims, path_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::articles::DeleteArticleResponse::Status200_NoContent => {
                let mut response = response.status(200);
                response.body(Body::empty())
            }
            apis::articles::DeleteArticleResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::articles::DeleteArticleResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_article_validation(
    path_params: models::GetArticlePathParams,
) -> std::result::Result<(models::GetArticlePathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// GetArticle - GET /api/articles/{slug}
#[tracing::instrument(skip_all)]
async fn get_article<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    Path(path_params): Path<models::GetArticlePathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::articles::Articles<Claims = C>,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_article_validation(path_params))
        .await
        .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .get_article(method, host, cookies, path_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::articles::GetArticleResponse::Status200_SingleArticle(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::articles::GetArticleResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_articles_validation(
    query_params: models::GetArticlesQueryParams,
) -> std::result::Result<(models::GetArticlesQueryParams,), ValidationErrors> {
    query_params.validate()?;

    Ok((query_params,))
}
/// GetArticles - GET /api/articles
#[tracing::instrument(skip_all)]
async fn get_articles<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    Query(query_params): Query<models::GetArticlesQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::articles::Articles<Claims = C>,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_articles_validation(query_params))
        .await
        .unwrap();

    let Ok((query_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .get_articles(method, host, cookies, query_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::articles::GetArticlesResponse::Status200_MultipleArticles(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::articles::GetArticlesResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::articles::GetArticlesResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_articles_feed_validation(
    query_params: models::GetArticlesFeedQueryParams,
) -> std::result::Result<(models::GetArticlesFeedQueryParams,), ValidationErrors> {
    query_params.validate()?;

    Ok((query_params,))
}
/// GetArticlesFeed - GET /api/articles/feed
#[tracing::instrument(skip_all)]
async fn get_articles_feed<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    Query(query_params): Query<models::GetArticlesFeedQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::articles::Articles<Claims = C> + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation =
        tokio::task::spawn_blocking(move || get_articles_feed_validation(query_params))
            .await
            .unwrap();

    let Ok((query_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .get_articles_feed(method, host, cookies, claims, query_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::articles::GetArticlesFeedResponse::Status200_MultipleArticles(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::articles::GetArticlesFeedResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::articles::GetArticlesFeedResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct UpdateArticleBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::UpdateArticleRequest,
}

#[tracing::instrument(skip_all)]
fn update_article_validation(
    path_params: models::UpdateArticlePathParams,
    body: models::UpdateArticleRequest,
) -> std::result::Result<
    (
        models::UpdateArticlePathParams,
        models::UpdateArticleRequest,
    ),
    ValidationErrors,
> {
    path_params.validate()?;
    let b = UpdateArticleBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// UpdateArticle - PUT /api/articles/{slug}
#[tracing::instrument(skip_all)]
async fn update_article<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::UpdateArticlePathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::UpdateArticleRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::articles::Articles<Claims = C> + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation =
        tokio::task::spawn_blocking(move || update_article_validation(path_params, body))
            .await
            .unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .update_article(method, host, cookies, claims, path_params, body)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::articles::UpdateArticleResponse::Status200_SingleArticle(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::articles::UpdateArticleResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::articles::UpdateArticleResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct CreateArticleCommentBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::CreateArticleCommentRequest,
}

#[tracing::instrument(skip_all)]
fn create_article_comment_validation(
    path_params: models::CreateArticleCommentPathParams,
    body: models::CreateArticleCommentRequest,
) -> std::result::Result<
    (
        models::CreateArticleCommentPathParams,
        models::CreateArticleCommentRequest,
    ),
    ValidationErrors,
> {
    path_params.validate()?;
    let b = CreateArticleCommentBodyValidator { body: &body };
    b.validate()?;

    Ok((path_params, body))
}
/// CreateArticleComment - POST /api/articles/{slug}/comments
#[tracing::instrument(skip_all)]
async fn create_article_comment<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::CreateArticleCommentPathParams>,
    State(api_impl): State<I>,
    Json(body): Json<models::CreateArticleCommentRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::comments::Comments<Claims = C> + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation =
        tokio::task::spawn_blocking(move || create_article_comment_validation(path_params, body))
            .await
            .unwrap();

    let Ok((path_params, body)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .create_article_comment(method, host, cookies, claims, path_params, body)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::comments::CreateArticleCommentResponse::Status200_SingleComment(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::comments::CreateArticleCommentResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::comments::CreateArticleCommentResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn delete_article_comment_validation(
    path_params: models::DeleteArticleCommentPathParams,
) -> std::result::Result<(models::DeleteArticleCommentPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// DeleteArticleComment - DELETE /api/articles/{slug}/comments/{id}
#[tracing::instrument(skip_all)]
async fn delete_article_comment<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::DeleteArticleCommentPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::comments::Comments<Claims = C> + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation =
        tokio::task::spawn_blocking(move || delete_article_comment_validation(path_params))
            .await
            .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .delete_article_comment(method, host, cookies, claims, path_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::comments::DeleteArticleCommentResponse::Status200_NoContent => {
                let mut response = response.status(200);
                response.body(Body::empty())
            }
            apis::comments::DeleteArticleCommentResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::comments::DeleteArticleCommentResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_article_comments_validation(
    path_params: models::GetArticleCommentsPathParams,
) -> std::result::Result<(models::GetArticleCommentsPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// GetArticleComments - GET /api/articles/{slug}/comments
#[tracing::instrument(skip_all)]
async fn get_article_comments<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    Path(path_params): Path<models::GetArticleCommentsPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::comments::Comments<Claims = C>,
{
    #[allow(clippy::redundant_closure)]
    let validation =
        tokio::task::spawn_blocking(move || get_article_comments_validation(path_params))
            .await
            .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .get_article_comments(method, host, cookies, path_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::comments::GetArticleCommentsResponse::Status200_MultipleComments(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::comments::GetArticleCommentsResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::comments::GetArticleCommentsResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn create_article_favorite_validation(
    path_params: models::CreateArticleFavoritePathParams,
) -> std::result::Result<(models::CreateArticleFavoritePathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// CreateArticleFavorite - POST /api/articles/{slug}/favorite
#[tracing::instrument(skip_all)]
async fn create_article_favorite<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::CreateArticleFavoritePathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::favorites::Favorites<Claims = C> + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation =
        tokio::task::spawn_blocking(move || create_article_favorite_validation(path_params))
            .await
            .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .create_article_favorite(method, host, cookies, claims, path_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::favorites::CreateArticleFavoriteResponse::Status200_SingleArticle(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::favorites::CreateArticleFavoriteResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::favorites::CreateArticleFavoriteResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn delete_article_favorite_validation(
    path_params: models::DeleteArticleFavoritePathParams,
) -> std::result::Result<(models::DeleteArticleFavoritePathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// DeleteArticleFavorite - DELETE /api/articles/{slug}/favorite
#[tracing::instrument(skip_all)]
async fn delete_article_favorite<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::DeleteArticleFavoritePathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::favorites::Favorites<Claims = C> + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation =
        tokio::task::spawn_blocking(move || delete_article_favorite_validation(path_params))
            .await
            .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .delete_article_favorite(method, host, cookies, claims, path_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::favorites::DeleteArticleFavoriteResponse::Status200_SingleArticle(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::favorites::DeleteArticleFavoriteResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::favorites::DeleteArticleFavoriteResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn follow_user_by_username_validation(
    path_params: models::FollowUserByUsernamePathParams,
) -> std::result::Result<(models::FollowUserByUsernamePathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// FollowUserByUsername - POST /api/profiles/{username}/follow
#[tracing::instrument(skip_all)]
async fn follow_user_by_username<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::FollowUserByUsernamePathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::profile::Profile<Claims = C> + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation =
        tokio::task::spawn_blocking(move || follow_user_by_username_validation(path_params))
            .await
            .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .follow_user_by_username(method, host, cookies, claims, path_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::profile::FollowUserByUsernameResponse::Status200_Profile(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::profile::FollowUserByUsernameResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::profile::FollowUserByUsernameResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_profile_by_username_validation(
    path_params: models::GetProfileByUsernamePathParams,
) -> std::result::Result<(models::GetProfileByUsernamePathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// GetProfileByUsername - GET /api/profiles/{username}
#[tracing::instrument(skip_all)]
async fn get_profile_by_username<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    Path(path_params): Path<models::GetProfileByUsernamePathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::profile::Profile<Claims = C>,
{
    #[allow(clippy::redundant_closure)]
    let validation =
        tokio::task::spawn_blocking(move || get_profile_by_username_validation(path_params))
            .await
            .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .get_profile_by_username(method, host, cookies, path_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::profile::GetProfileByUsernameResponse::Status200_Profile(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::profile::GetProfileByUsernameResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::profile::GetProfileByUsernameResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn unfollow_user_by_username_validation(
    path_params: models::UnfollowUserByUsernamePathParams,
) -> std::result::Result<(models::UnfollowUserByUsernamePathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// UnfollowUserByUsername - DELETE /api/profiles/{username}/follow
#[tracing::instrument(skip_all)]
async fn unfollow_user_by_username<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    Path(path_params): Path<models::UnfollowUserByUsernamePathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::profile::Profile<Claims = C> + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation =
        tokio::task::spawn_blocking(move || unfollow_user_by_username_validation(path_params))
            .await
            .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .unfollow_user_by_username(method, host, cookies, claims, path_params)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::profile::UnfollowUserByUsernameResponse::Status200_Profile(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::profile::UnfollowUserByUsernameResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::profile::UnfollowUserByUsernameResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_tags_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// GetTags - GET /api/tags
#[tracing::instrument(skip_all)]
async fn get_tags<I, A>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::tags::Tags,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_tags_validation())
        .await
        .unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().get_tags(method, host, cookies).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::tags::GetTagsResponse::Status200_Tags(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tags::GetTagsResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct CreateUserBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::CreateUserRequest,
}

#[tracing::instrument(skip_all)]
fn create_user_validation(
    body: models::CreateUserRequest,
) -> std::result::Result<(models::CreateUserRequest,), ValidationErrors> {
    let b = CreateUserBodyValidator { body: &body };
    b.validate()?;

    Ok((body,))
}
/// CreateUser - POST /api/users
#[tracing::instrument(skip_all)]
async fn create_user<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    State(api_impl): State<I>,
    Json(body): Json<models::CreateUserRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::user_and_authentication::UserAndAuthentication<Claims = C>,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || create_user_validation(body))
        .await
        .unwrap();

    let Ok((body,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .create_user(method, host, cookies, body)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::user_and_authentication::CreateUserResponse::Status201_User(body) => {
                let mut response = response.status(201);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::user_and_authentication::CreateUserResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_current_user_validation() -> std::result::Result<(), ValidationErrors> {
    Ok(())
}
/// GetCurrentUser - GET /api/user
#[tracing::instrument(skip_all)]
async fn get_current_user<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::user_and_authentication::UserAndAuthentication<Claims = C>
        + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_current_user_validation())
        .await
        .unwrap();

    let Ok(()) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .get_current_user(method, host, cookies, claims)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::user_and_authentication::GetCurrentUserResponse::Status200_User(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::user_and_authentication::GetCurrentUserResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::user_and_authentication::GetCurrentUserResponse::Status422_UnexpectedError(
                body,
            ) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct LoginBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::LoginRequest,
}

#[tracing::instrument(skip_all)]
fn login_validation(
    body: models::LoginRequest,
) -> std::result::Result<(models::LoginRequest,), ValidationErrors> {
    let b = LoginBodyValidator { body: &body };
    b.validate()?;

    Ok((body,))
}
/// Login - POST /api/users/login
#[tracing::instrument(skip_all)]
async fn login<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    State(api_impl): State<I>,
    Json(body): Json<models::LoginRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::user_and_authentication::UserAndAuthentication<Claims = C>,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || login_validation(body))
        .await
        .unwrap();

    let Ok((body,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl.as_ref().login(method, host, cookies, body).await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::user_and_authentication::LoginResponse::Status200_User(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::user_and_authentication::LoginResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::user_and_authentication::LoginResponse::Status422_UnexpectedError(body) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(validator::Validate)]
#[allow(dead_code)]
struct UpdateCurrentUserBodyValidator<'a> {
    #[validate(nested)]
    body: &'a models::UpdateCurrentUserRequest,
}

#[tracing::instrument(skip_all)]
fn update_current_user_validation(
    body: models::UpdateCurrentUserRequest,
) -> std::result::Result<(models::UpdateCurrentUserRequest,), ValidationErrors> {
    let b = UpdateCurrentUserBodyValidator { body: &body };
    b.validate()?;

    Ok((body,))
}
/// UpdateCurrentUser - PUT /api/user
#[tracing::instrument(skip_all)]
async fn update_current_user<I, A, C>(
    method: Method,
    host: Host,
    cookies: CookieJar,
    headers: HeaderMap,
    State(api_impl): State<I>,
    Json(body): Json<models::UpdateCurrentUserRequest>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::user_and_authentication::UserAndAuthentication<Claims = C>
        + apis::ApiKeyAuthHeader<Claims = C>,
{
    // Authentication
    let claims_in_header = api_impl
        .as_ref()
        .extract_claims_from_header(&headers, "Authorization")
        .await;
    let claims = None.or(claims_in_header);
    let Some(claims) = claims else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    };

    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || update_current_user_validation(body))
        .await
        .unwrap();

    let Ok((body,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .update_current_user(method, host, cookies, claims, body)
        .await;

    let mut response = Response::builder();

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::user_and_authentication::UpdateCurrentUserResponse::Status200_User(body) => {
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::user_and_authentication::UpdateCurrentUserResponse::Status401_Unauthorized => {
                let mut response = response.status(401);
                response.body(Body::empty())
            }
            apis::user_and_authentication::UpdateCurrentUserResponse::Status422_UnexpectedError(
                body,
            ) => {
                let mut response = response.status(422);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_str("application/json").map_err(|e| {
                            error!(error = ?e);
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(_) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            response.status(500).body(Body::empty())
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
