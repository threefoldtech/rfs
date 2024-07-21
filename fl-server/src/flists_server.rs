use askama::Template;
use axum::{
    response::{Html, Response},
    Extension, Json,
};
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf};
use tokio::io;
use tower::util::ServiceExt;
use tower_http::services::ServeDir;
use utoipa::ToSchema;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use axum_macros::debug_handler;
use percent_encoding::percent_decode;

use crate::config;

#[utoipa::path(
	get,
	path = "/v1/api/fl",
	responses(
        (status = 200, description = "listing flists", body = Vec<FileInfo>),
        (status = 40x, description = "listing flists error", body = ResponseError)
	),
	params(
		("path" = String, Path, description = "flist path")
	)
)]
#[debug_handler]
pub async fn list_flists_handler(Extension(cfg): Extension<config::Config>) -> impl IntoResponse {
    let mut flists: HashMap<String, Vec<FileInfo>> = HashMap::new();

    let rs = visit_dir_one_level(std::path::Path::new(&cfg.flist_dir)).await;
    match rs {
        Ok(files) => {
            for file in files {
                if !file.is_file {
                    let flists_per_username =
                        visit_dir_one_level(std::path::Path::new(&file.path_uri)).await;
                    match flists_per_username {
                        Ok(files) => flists.insert(file.name, files),
                        Err(e) => {
                            log::error!("failed to list flists per username with error: {}", e);
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({
                                    "msg": "Internal server error",
                                })),
                            );
                        }
                    };
                };
            }
        }
        Err(e) => {
            log::error!("failed to list flists directory with error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "msg": "Internal server error",
                })),
            );
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "flists": flists,
        })),
    )
}

#[utoipa::path(
	get,
	path = "/{path}",
	responses(
        (status = 200, description = "listing flists", body = Vec<FileInfo>),
        (status = 40x, description = "listing flists error", body = ResponseError)
	),
	params(
		("path" = String, Path, description = "flist path")
	)
)]
#[debug_handler]
pub async fn serve_flists(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().to_string();

    return match ServeDir::new("").oneshot(req).await {
        Ok(res) => {
            let status = res.status();
            match status {
                StatusCode::NOT_FOUND => {
                    let path = path.trim_start_matches('/');
                    let path = percent_decode(path.as_ref()).decode_utf8_lossy();

                    let mut full_path = PathBuf::new();

                    // validate
                    for seg in path.split('/') {
                        if seg.starts_with("..") || seg.contains('\\') {
                            return Err(ErrorTemplate {
                                err: ResponseError::BadRequest("invalid path".to_string()),
                                cur_path: path.to_string(),
                                message: "invalid path".to_owned(),
                            });
                        }
                        full_path.push(seg);
                    }

                    let cur_path = std::path::Path::new(&full_path);

                    match cur_path.is_dir() {
                        true => {
                            let rs = visit_dir_one_level(&full_path).await;
                            match rs {
                                Ok(files) => Ok(DirListTemplate {
                                    lister: DirLister { files },
                                    cur_path: path.to_string(),
                                }
                                .into_response()),
                                Err(e) => Err(ErrorTemplate {
                                    err: ResponseError::InternalError(e.to_string()),
                                    cur_path: path.to_string(),
                                    message: e.to_string(),
                                }),
                            }
                        }
                        false => Err(ErrorTemplate {
                            err: ResponseError::FileNotFound("file not found".to_string()),
                            cur_path: path.to_string(),
                            message: "file not found".to_owned(),
                        }),
                    }
                }
                _ => Ok(res.map(axum::body::Body::new)),
            }
        }
        Err(err) => Err(ErrorTemplate {
            err: ResponseError::InternalError(format!("Unhandled error: {}", err)),
            cur_path: path.to_string(),
            message: format!("Unhandled error: {}", err),
        }),
    };
}

async fn visit_dir_one_level(path: &std::path::Path) -> io::Result<Vec<FileInfo>> {
    let mut dir = tokio::fs::read_dir(path).await?;
    let mut files: Vec<FileInfo> = Vec::new();

    while let Some(child) = dir.next_entry().await? {
        let the_uri_path = child.path().to_string_lossy().to_string();

        files.push(FileInfo {
            name: child.file_name().to_string_lossy().to_string(),
            path_uri: the_uri_path,
            is_file: child.file_type().await?.is_file(),
            last_modified: child
                .metadata()
                .await?
                .modified()?
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        });
    }

    Ok(files)
}

mod filters {
    pub(crate) fn datetime(ts: &i64) -> ::askama::Result<String> {
        if let Ok(format) =
            time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second] UTC")
        {
            return Ok(time::OffsetDateTime::from_unix_timestamp(*ts)
                .unwrap()
                .format(&format)
                .unwrap());
        }
        Err(askama::Error::Fmt(std::fmt::Error))
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct DirListTemplate {
    lister: DirLister,
    cur_path: String,
}

impl IntoResponse for DirListTemplate {
    fn into_response(self) -> Response<Body> {
        let t = self;
        match t.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => {
                tracing::error!("template render failed, err={}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to render template. Error: {}", err),
                )
                    .into_response()
            }
        }
    }
}

struct DirLister {
    files: Vec<FileInfo>,
}

#[derive(Serialize)]
struct FileInfo {
    name: String,
    path_uri: String,
    is_file: bool,
    last_modified: i64,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    err: ResponseError,
    cur_path: String,
    message: String,
}

const FAIL_REASON_HEADER_NAME: &str = "fl-server-fail-reason";

impl IntoResponse for ErrorTemplate {
    fn into_response(self) -> Response<Body> {
        let t = self;
        match t.render() {
            Ok(html) => {
                let mut resp = Html(html).into_response();
                match t.err {
                    ResponseError::FileNotFound(reason) => {
                        *resp.status_mut() = StatusCode::NOT_FOUND;
                        resp.headers_mut()
                            .insert(FAIL_REASON_HEADER_NAME, reason.parse().unwrap());
                    }
                    ResponseError::BadRequest(reason) => {
                        *resp.status_mut() = StatusCode::BAD_REQUEST;
                        resp.headers_mut()
                            .insert(FAIL_REASON_HEADER_NAME, reason.parse().unwrap());
                    }
                    ResponseError::InternalError(reason) => {
                        *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        resp.headers_mut()
                            .insert(FAIL_REASON_HEADER_NAME, reason.parse().unwrap());
                    }
                }
                resp
            }
            Err(err) => {
                tracing::error!("template render failed, err={}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to render template. Error: {}", err),
                )
                    .into_response()
            }
        }
    }
}

#[derive(ToSchema)]
enum ResponseError {
    BadRequest(String),
    FileNotFound(String),
    InternalError(String),
}
