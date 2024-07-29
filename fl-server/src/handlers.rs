use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use axum_macros::debug_handler;
use std::{
    collections::HashMap,
    fs,
    sync::{mpsc, Arc},
};
use tokio::io;

use bollard::auth::DockerCredentials;
use serde::{Deserialize, Serialize};

use crate::auth::{SignInData, __path_sign_in_handler};
use crate::{
    config::{self, AppState, Job},
    flists_server::{visit_dir_one_level, FileInfo},
};
use rfs::fungi::Writer;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(
    paths(health_check_handler, create_flist_handler, get_flist_state_handler, list_flists_handler, sign_in_handler),
    components(schemas(FlistInputs, Job, ResponseError, FileInfo, AppState, SignInData)),
    tags(
        (name = "fl-server", description = "Flist conversion API")
    )
)]
pub struct FlistApi;

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct FlistInputs {
    #[schema(example = "redis")]
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
    Accepted(String),
    Started(String),
    InProgress(FlistStateInfo),
    Created(String),
    Failed(String),
    NotExists(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct FlistStateInfo {
    msg: String,
    progress: f32,
}

#[utoipa::path(
    get,
    path = "/v1/api",
    responses(
        (status = 200, description = "flist health check")
    )
)]
pub async fn health_check_handler() -> impl IntoResponse {
    let json_response = serde_json::json!({
        "msg": "flist health check"
    });

    (StatusCode::OK, Json(json_response))
}

#[utoipa::path(
    post,
    path = "/v1/api/fl",
    request_body = FlistInputs,
    responses(
        (status = 201, description = "Flist conversion started", body = Job),
        (status = 500, description = "Internal server error", body = ResponseError),
        (status = 401, description = "Unauthorized user"),
        (status = 403, description = "Forbidden"),
    )
)]
#[debug_handler]
pub async fn create_flist_handler(
    State(state): State<Arc<config::AppState>>,
    Extension(cfg): Extension<config::Config>,
    Extension(username): Extension<String>,
    Json(body): Json<FlistInputs>,
) -> impl IntoResponse {
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
    let username_dir = format!("{}/{}", cfg.flist_dir, username);

    match flist_exists(std::path::Path::new(&username_dir), &fl_name).await {
        Ok(exists) => {
            if exists {
                return (
                    StatusCode::CONFLICT,
                    Json(ResponseError::Conflict("flist already exists".to_string())),
                )
                    .into_response();
            }
        }
        Err(e) => {
            log::error!("failed to check flist existence with error {:?}", e);

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ResponseError::InternalServerError(
                    "internal server error".to_string(),
                )),
            )
                .into_response();
        }
    }

    let created = fs::create_dir_all(&username_dir);
    if created.is_err() {
        log::error!(
            "failed to create user flist directory `{}` with error {:?}",
            &username_dir,
            created.err()
        );

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ResponseError::InternalServerError(
                "internal server error".to_string(),
            )),
        )
            .into_response();
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

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ResponseError::InternalServerError(
                    "internal server error".to_string(),
                )),
            )
                .into_response();
        }
    };

    let store = match rfs::store::parse_router(&cfg.store_url).await {
        Ok(s) => s,
        Err(err) => {
            log::error!("failed to parse router for store with error {}", err);

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ResponseError::InternalServerError(
                    "internal server error".to_string(),
                )),
            )
                .into_response();
        }
    };

    // Create a new job id for the flist request
    let job: Job = Job {
        id: Uuid::new_v4().to_string(),
    };
    let current_job = job.clone();

    state.jobs_state.lock().unwrap().insert(
        job.id.clone(),
        FlistState::Accepted(format!("flist '{}' is accepted", fl_name)),
    );

    tokio::spawn(async move {
        state.jobs_state.lock().unwrap().insert(
            job.id.clone(),
            FlistState::Started(format!("flist '{}' is started", fl_name)),
        );

        let container_name = Uuid::new_v4().to_string();
        let docker_tmp_dir = tempdir::TempDir::new(&container_name).unwrap();
        let docker_tmp_dir_path = docker_tmp_dir.path().to_owned();

        let (tx, rx) = mpsc::channel();
        let mut docker_to_fl = docker2fl::DockerImageToFlist::new(
            meta,
            docker_image,
            credentials,
            docker_tmp_dir_path.clone(),
        );

        let res = docker_to_fl.prepare().await;
        if res.is_err() {
            state.jobs_state.lock().unwrap().insert(
                job.id.clone(),
                FlistState::Failed(format!("flist preparing '{}' has failed", fl_name)),
            );
            return;
        }

        let files_count = docker_to_fl.files_count();
        let st = state.clone();
        let job_id = job.id.clone();
        tokio::spawn(async move {
            let mut progress: f32 = 0.0;

            for _ in 0..files_count - 1 {
                let step = rx.recv().unwrap() as f32;
                progress += step;
                st.jobs_state.lock().unwrap().insert(
                    job_id.clone(),
                    FlistState::InProgress(FlistStateInfo {
                        msg: "flist is in progress".to_string(),
                        progress: progress / files_count as f32 * 100.0,
                    }),
                );
            }
        });

        let res = docker_to_fl.pack(store, Some(tx)).await;

        // remove the file created with the writer if fl creation failed
        if res.is_err() {
            let _ = tokio::fs::remove_file(&fl_path).await;
            state.jobs_state.lock().unwrap().insert(
                job.id.clone(),
                FlistState::Failed(format!("flist '{}' has failed", fl_name)),
            );
            return;
        }

        state.jobs_state.lock().unwrap().insert(
            job.id.clone(),
            FlistState::Created(format!(
                "flist {}:{}/{}/{}/{} is created successfully",
                cfg.host, cfg.port, cfg.flist_dir, username, fl_name
            )),
        );
    });

    (StatusCode::CREATED, Json(serde_json::json!(current_job))).into_response()
}

#[utoipa::path(
    get,
    path = "/v1/api/fl/{job_id}",
    responses(
        (status = 200, description = "flist state", body = AppState),
        (status = 401, description = "Unauthorized user"),
        (status = 404, description = "flist not found", body = AppState),
        (status = 403, description = "Forbidden"),
    ),
    params(
        ("job_id" = String, Path, description = "flist job id")
    )
)]
#[debug_handler]
pub async fn get_flist_state_handler(
    Path(flist_job_id): Path<String>,
    State(state): State<Arc<config::AppState>>,
) -> impl IntoResponse {
    if !&state
        .jobs_state
        .lock()
        .unwrap()
        .contains_key(&flist_job_id.clone())
    {
        return (
            StatusCode::NOT_FOUND,
            Json(FlistState::NotExists("flist doesn't exist".to_string())),
        )
            .into_response();
    }

    // if flist creation failed or done clean it from the state
    // TODO: clean if done or error
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "flist_state": &state.jobs_state.lock().unwrap().get(&flist_job_id.clone())
        })),
    )
        .into_response()
}

#[utoipa::path(
	get,
	path = "/v1/api/fl",
	responses(
        (status = 200, description = "Listing flists", body = HashMap<String, Vec<FileInfo>>),
        (status = 500, description = "Internal server error", body = ResponseError),
        (status = 401, description = "Unauthorized user"),
        (status = 403, description = "Forbidden"),
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
                                Json(ResponseError::InternalServerError(
                                    "internal server error".to_string(),
                                )),
                            )
                                .into_response();
                        }
                    };
                };
            }
        }
        Err(e) => {
            log::error!("failed to list flists directory with error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ResponseError::InternalServerError(
                    "internal server error".to_string(),
                )),
            )
                .into_response();
        }
    }

    (StatusCode::OK, Json(serde_json::json!(flists))).into_response()
}

pub async fn flist_exists(dir_path: &std::path::Path, flist_name: &String) -> io::Result<bool> {
    let mut dir = tokio::fs::read_dir(dir_path).await?;

    while let Some(child) = dir.next_entry().await? {
        let file_name = child.file_name().to_string_lossy().to_string();

        if file_name.eq(flist_name) {
            return Ok(true);
        }
    }

    Ok(false)
}

#[derive(Serialize, Deserialize, ToSchema)]
enum ResponseError {
    #[schema(example = "internal server error")]
    InternalServerError(String),
    #[schema(example = "flist already exists")]
    Conflict(String),
    #[schema(example = "flist path not found")]
    NotFound(String),
    #[schema(example = "token is missing")]
    Unauthorized(String),
    #[schema(example = "user bad request")]
    BadRequest(String),
}
