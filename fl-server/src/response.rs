use std::collections::HashMap;

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{auth::SignInResponse, config::Job, handlers::FlistState, serve_flists::FileInfo};

#[derive(Serialize, Deserialize, ToSchema)]
pub enum ResponseError {
    InternalServerError,
    Conflict(String),
    NotFound(String),
    Unauthorized(String),
    BadRequest(String),
    Forbidden(String),
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> Response<Body> {
        match self {
            ResponseError::InternalServerError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            ResponseError::Conflict(msg) => (StatusCode::CONFLICT, msg).into_response(),
            ResponseError::NotFound(msg) => (StatusCode::NOT_FOUND, msg).into_response(),
            ResponseError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg).into_response(),
            ResponseError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            ResponseError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg).into_response(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub enum ResponseResult {
    Health,
    FlistCreated(Job),
    FlistState(FlistState),
    Flists(HashMap<String, Vec<FileInfo>>),
    SignedIn(SignInResponse),
}

impl IntoResponse for ResponseResult {
    fn into_response(self) -> Response<Body> {
        match self {
            ResponseResult::Health => (
                StatusCode::OK,
                Json(serde_json::json!({"msg": "flist server is working"})),
            )
                .into_response(),
            ResponseResult::SignedIn(token) => {
                (StatusCode::CREATED, Json(serde_json::json!(token))).into_response()
            }
            ResponseResult::FlistCreated(job) => (StatusCode::CREATED, Json(job)).into_response(),
            ResponseResult::FlistState(flist_state) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "flist_state": flist_state
                })),
            )
                .into_response(),
            ResponseResult::Flists(flists) => (StatusCode::OK, Json(flists)).into_response(),
        }
    }
}
