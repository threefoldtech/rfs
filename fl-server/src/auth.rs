use axum::{
    body::Body,
    extract::{Json, Request, State},
    http::{self, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
};
use axum_macros::debug_handler;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::config;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,       // Expiry time of the token
    pub iat: usize,       // Issued at time of the token
    pub username: String, // Username associated with the token
}

#[derive(Deserialize, ToSchema)]
pub struct SignInData {
    pub username: String,
    pub password: String,
}

#[utoipa::path(
    post,
    path = "/v1/api/signin",
    request_body = SignInData,
    responses(
        (status = 200, description = "User signed in successfully", body = String),
        (status = 500, description = "Internal server error"),
        (status = 401, description = "Unauthorized user"),
    )
)]
#[debug_handler]
pub async fn sign_in_handler(
    Extension(cfg): Extension<config::Config>,
    Json(user_data): Json<SignInData>,
) -> Result<Json<String>, AuthError> {
    let user = match get_user_by_username(cfg.users, &user_data.username) {
        Some(user) => user,
        None => {
            return Err(AuthError {
                message: "User is not authorized".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            })
        }
    };

    if &user_data.password != &user.password {
        return Err(AuthError {
            message: "Wrong username or password".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        });
    }

    let token =
        encode_jwt(user.username, cfg.jwt_secret, cfg.jwt_expire_hours).map_err(|_| AuthError {
            message: "Internal server error".to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(Json(token))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
}

fn get_user_by_username(users: Vec<User>, username: &str) -> Option<User> {
    let user = users.iter().find(|u| u.username == username)?;
    Some(user.clone())
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

#[derive(ToSchema)]
pub struct AuthError {
    // TODO:
    message: String,
    status_code: StatusCode,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response<Body> {
        let t = self;
        (t.status_code, t.message).into_response()
    }
}

pub async fn authorize(
    State(cfg): State<config::Config>,
    mut req: Request,
    next: Next,
) -> Result<Response<Body>, AuthError> {
    let auth_header = match req.headers_mut().get(http::header::AUTHORIZATION) {
        Some(header) => header.to_str().map_err(|_| AuthError {
            message: "Empty header is not allowed".to_string(),
            status_code: StatusCode::FORBIDDEN,
        })?,
        None => {
            return Err(AuthError {
                message: "No JWT token is added to the header".to_string(),
                status_code: StatusCode::FORBIDDEN,
            })
        }
    };

    let mut header = auth_header.split_whitespace();
    let (_, token) = (header.next(), header.next());
    let token_data = match decode_jwt(token.unwrap().to_string(), cfg.jwt_secret) {
        Ok(data) => data,
        Err(_) => {
            return Err(AuthError {
                message: "Unable to decode JWT token".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            })
        }
    };

    let current_user = match get_user_by_username(cfg.users, &token_data.claims.username) {
        Some(user) => user,
        None => {
            return Err(AuthError {
                message: "You are not an authorized user".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            })
        }
    };

    req.extensions_mut().insert(current_user.username);
    Ok(next.run(req).await)
}
