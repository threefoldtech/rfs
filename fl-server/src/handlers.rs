use askama::Template;
use axum::response::{Html, Response};
use std::{fs, path::PathBuf, sync::Arc};
use tokio::io;
use tower::util::ServiceExt;
use tower_http::services::ServeDir;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use axum_macros::debug_handler;

use bollard::auth::DockerCredentials;
use percent_encoding::percent_decode;
use serde::{Deserialize, Serialize};

use rfs::fungi::Writer;
use uuid::Uuid;

use crate::config::{self, JobID};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FlistInputs {
    pub image_name: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub auth: Option<String>,
    pub email: Option<String>,
    pub server_address: Option<String>,
    pub identity_token: Option<String>,
    pub registry_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum FlistState {
    Accepted,
    Started,
    Created,
    Failed,
    NotExists,
}

pub async fn health_check_handler() -> impl IntoResponse {
    let json_response = serde_json::json!({
        "status": "success",
        "message": "flist health check"
    });

    (StatusCode::OK, Json(json_response))
}

#[debug_handler]
pub async fn create_flist_handler(
    State(state): State<Arc<config::AppState>>,
    Extension(config): Extension<config::Config>,
    Extension(username): Extension<String>,
    Json(body): Json<FlistInputs>,
) -> Result<String, StatusCode> {
    let credentials = Some(DockerCredentials {
        username: body.username,
        password: body.password,
        auth: body.auth,
        email: body.email,
        serveraddress: body.server_address,
        identitytoken: body.identity_token,
        registrytoken: body.registry_token,
    });

    let mut docker_image = body.image_name.to_string();
    if !docker_image.contains(':') {
        docker_image.push_str(":latest");
    }

    let fl_name = docker_image.replace([':', '/'], "-") + ".fl";
    let username_dir = format!("{}/{}", config.flist_dir, username);
    let created = fs::create_dir_all(&username_dir);
    if created.is_err() {
        log::error!(
            "failed to create user flist directory `{}` with error {:?}",
            &username_dir,
            created.err()
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let fl_path: String = format!("{}/{}", username_dir, fl_name);

    let meta = match Writer::new(&fl_path).await {
        Ok(writer) => writer,
        Err(err) => {
            log::error!(
                "failed to create a new writer for flist `{}` with error {}",
                fl_path,
                err
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let store = match rfs::store::parse_router(&config.store_url).await {
        Ok(s) => s,
        Err(err) => {
            log::error!("failed to parse router for store with error {}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Create a new job id for the flist request
    let job_id = JobID(Uuid::new_v4().to_string());
    let cloned_job_id = job_id.clone();

    state
        .jobs_state
        .lock()
        .unwrap()
        .insert(cloned_job_id.clone(), FlistState::Accepted);

    tokio::spawn(async move {
        state
            .jobs_state
            .lock()
            .unwrap()
            .insert(cloned_job_id.clone(), FlistState::Started);

        let res = docker2fl::convert(meta, store, &docker_image, credentials).await;

        // remove the file created with the writer if fl creation failed
        if res.is_err() {
            let _ = tokio::fs::remove_file(&fl_path).await;
            state
                .jobs_state
                .lock()
                .unwrap()
                .insert(cloned_job_id.clone(), FlistState::Failed);
        }

        state
            .jobs_state
            .lock()
            .unwrap()
            .insert(cloned_job_id.clone(), FlistState::Created);
    });

    Ok(job_id.0)
}

#[debug_handler]
pub async fn get_flist_state_handler(
    Path(flist_job_id): Path<String>,
    State(state): State<Arc<config::AppState>>,
) -> impl IntoResponse {
    // flist job ID doesn't exits
    if !&state
        .jobs_state
        .lock()
        .unwrap()
        .contains_key(&JobID(flist_job_id.clone()))
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "status": "failed",
                "message": FlistState::NotExists,
            })),
        );
    }

    // if flist creation failed or done clean it from the state
    // TODO: clean if done or error
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "success",
            "job_state": &state.jobs_state.lock().unwrap().get(&JobID(flist_job_id.clone())),
        })),
    )
}

// TODO: add auth to templates
#[debug_handler]
pub async fn get_flists_handler(
    Extension(username): Extension<String>,
    req: Request<Body>,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();

    let mut splitted_path = path.split("/");
    let (_, _, path_username) = (
        splitted_path.next(),
        splitted_path.next(),
        splitted_path.next(),
    );

    if path_username.unwrap() != username {
        return Err(ErrorTemplate {
            err: ResponseError::Unauthorized(
                "You are not authorized to the specified path".to_string(),
            ),
            cur_path: path.to_string(),
            message: "You are not authorized to the specified path".to_string(),
        });
    }

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
                    ResponseError::Unauthorized(reason) => {
                        *resp.status_mut() = StatusCode::UNAUTHORIZED;
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

enum ResponseError {
    BadRequest(String),
    FileNotFound(String),
    InternalError(String),
    Unauthorized(String),
}
