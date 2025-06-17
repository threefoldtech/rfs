use std::sync::Arc;

use axum::{
    extract::{Json, Request, State},
    http::{self, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use axum_macros::debug_handler;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    config,
    db::DB,
    response::{ResponseError, ResponseResult},
};

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,       // Expiry time of the token
    pub iat: usize,       // Issued at time of the token
    pub username: String, // Username associated with the token
}

#[derive(Deserialize, ToSchema)]
pub struct SignInBody {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct SignInResponse {
    pub access_token: String,
}

#[utoipa::path(
    post,
    path = "/v1/api/signin",
    request_body = SignInBody,
    responses(
        (status = 200, description = "User signed in successfully", body = SignInResponse),
        (status = 500, description = "Internal server error"),
        (status = 401, description = "Unauthorized user"),
    )
)]
#[debug_handler]
pub async fn sign_in_handler(
    State(state): State<Arc<config::AppState>>,
    Json(user_data): Json<SignInBody>,
) -> impl IntoResponse {
    let user = match state.db.get_user_by_username(&user_data.username).await {
        Some(user) => user,
        None => {
            return Err(ResponseError::Unauthorized(
                "User is not authorized".to_string(),
            ));
        }
    };

    if user_data.password != user.password {
        return Err(ResponseError::Unauthorized(
            "Wrong username or password".to_string(),
        ));
    }

    let token = encode_jwt(
        user.username.clone(),
        state.config.jwt_secret.clone(),
        state.config.jwt_expire_hours,
    )
    .map_err(|_| ResponseError::InternalServerError)?;

    Ok(ResponseResult::SignedIn(SignInResponse {
        access_token: token,
    }))
}

pub fn encode_jwt(
    username: String,
    jwt_secret: String,
    jwt_expire: i64,
) -> Result<String, StatusCode> {
    let now = Utc::now();
    let exp: usize = (now + Duration::hours(jwt_expire)).timestamp() as usize;
    let iat: usize = now.timestamp() as usize;
    let claim = Claims { iat, exp, username };

    encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn decode_jwt(jwt_token: String, jwt_secret: String) -> Result<TokenData<Claims>, StatusCode> {
    let result: Result<TokenData<Claims>, StatusCode> = decode(
        &jwt_token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    result
}

pub async fn authorize(
    State(state): State<Arc<config::AppState>>,
    mut req: Request,
    next: Next,
) -> impl IntoResponse {
    let auth_header = match req.headers_mut().get(http::header::AUTHORIZATION) {
        Some(header) => header
            .to_str()
            .map_err(|_| ResponseError::Forbidden("Empty header is not allowed".to_string()))?,
        None => {
            return Err(ResponseError::Forbidden(
                "No JWT token is added to the header".to_string(),
            ))
        }
    };

    let mut header = auth_header.split_whitespace();
    let (_, token) = (header.next(), header.next());
    let token_str = match token {
        Some(t) => t.to_string(),
        None => {
            log::error!("failed to get token string");
            return Err(ResponseError::Unauthorized(
                "Authorization token is not provided".to_string(),
            ));
        }
    };

    let token_data = match decode_jwt(token_str, state.config.jwt_secret.clone()) {
        Ok(data) => data,
        Err(_) => {
            return Err(ResponseError::Forbidden(
                "Unable to decode JWT token".to_string(),
            ))
        }
    };

    let current_user = match state
        .db
        .get_user_by_username(&token_data.claims.username)
        .await
    {
        Some(user) => user,
        None => {
            return Err(ResponseError::Unauthorized(
                "You are not an authorized user".to_string(),
            ));
        }
    };

    req.extensions_mut().insert(current_user.username.clone());
    Ok(next.run(req).await)
}

/// Get the user ID from the username stored in the request extension
pub async fn get_user_id_from_token(db: &impl DB, username: &str) -> Result<i64, ResponseError> {
    match db.get_user_by_username(username).await {
        Some(user) => match user.id {
            Some(id) => Ok(id),
            None => {
                log::error!("User ID is missing for user: {}", username);
                Err(ResponseError::Unauthorized(
                    "User ID is missing".to_string(),
                ))
            }
        },
        None => {
            log::error!("User not found: {}", username);
            Err(ResponseError::Unauthorized("User not found".to_string()))
        }
    }
}
