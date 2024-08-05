use axum::{
    extract::{Path, State},
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

use crate::auth::{SignInBody, SignInResponse, __path_sign_in_handler};
use crate::{
    config::{self, Job},
    response::{ResponseError, ResponseResult},
    serve_flists::{visit_dir_one_level, FileInfo},
};
use rfs::fungi::Writer;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(
    paths(health_check_handler, create_flist_handler, get_flist_state_handler, list_flists_handler, sign_in_handler),
    components(schemas(FlistBody, Job, ResponseError, ResponseResult, FileInfo, SignInBody, FlistState, SignInResponse, FlistStateInfo)),
    tags(
        (name = "fl-server", description = "Flist conversion API")
    )
)]
pub struct FlistApi;

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct FlistBody {
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

#[derive(Debug, Clone, Serialize, PartialEq, ToSchema)]
pub enum FlistState {
    Accepted(String),
    Started(String),
    InProgress(FlistStateInfo),
    Created(String),
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, ToSchema)]
pub struct FlistStateInfo {
    msg: String,
    progress: f32,
}

#[utoipa::path(
    get,
    path = "/v1/api",
    responses(
        (status = 200, description = "flist server is working", body = String)
    )
)]
pub async fn health_check_handler() -> ResponseResult {
    ResponseResult::Health
}

#[utoipa::path(
    post,
    path = "/v1/api/fl",
    request_body = FlistBody,
    responses(
        (status = 201, description = "Flist conversion started", body = Job),
        (status = 401, description = "Unauthorized user"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Conflict"),
        (status = 500, description = "Internal server error"),
    )
)]
#[debug_handler]
pub async fn create_flist_handler(
    State(state): State<Arc<config::AppState>>,
    Extension(cfg): Extension<config::Config>,
    Extension(username): Extension<String>,
    Json(body): Json<FlistBody>,
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
                return Err(ResponseError::Conflict("flist already exists".to_string()));
            }
        }
        Err(e) => {
            log::error!("failed to check flist existence with error {:?}", e);
            return Err(ResponseError::InternalServerError);
        }
    }

    let created = fs::create_dir_all(&username_dir);
    if created.is_err() {
        log::error!(
            "failed to create user flist directory `{}` with error {:?}",
            &username_dir,
            created.err()
        );
        return Err(ResponseError::InternalServerError);
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
            return Err(ResponseError::InternalServerError);
        }
    };

    let store = match rfs::store::parse_router(&cfg.store_url).await {
        Ok(s) => s,
        Err(err) => {
            log::error!("failed to parse router for store with error {}", err);
            return Err(ResponseError::InternalServerError);
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
            state
                .jobs_state
                .lock()
                .unwrap()
                .insert(job.id.clone(), FlistState::Failed);
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
            log::error!("failed creation failed with error {:?}", res.err());
            let _ = tokio::fs::remove_file(&fl_path).await;
            state
                .jobs_state
                .lock()
                .unwrap()
                .insert(job.id.clone(), FlistState::Failed);
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

    Ok(ResponseResult::FlistCreated(current_job))
}

#[utoipa::path(
    get,
    path = "/v1/api/fl/{job_id}",
    responses(
        (status = 200, description = "Flist state", body = FlistState),
        (status = 404, description = "Flist not found"),
        (status = 500, description = "Internal server error"),
        (status = 401, description = "Unauthorized user"),
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
        return Err(ResponseError::NotFound("flist doesn't exist".to_string()));
    }

    let res_state = state
        .jobs_state
        .lock()
        .unwrap()
        .get(&flist_job_id.clone())
        .unwrap()
        .to_owned();

    match res_state {
        FlistState::Accepted(_) => Ok(ResponseResult::FlistState(res_state)),
        FlistState::Started(_) => Ok(ResponseResult::FlistState(res_state)),
        FlistState::InProgress(_) => Ok(ResponseResult::FlistState(res_state)),
        FlistState::Created(_) => {
            state
                .jobs_state
                .lock()
                .unwrap()
                .remove(&flist_job_id.clone());

            Ok(ResponseResult::FlistState(res_state))
        }
        FlistState::Failed => {
            state
                .jobs_state
                .lock()
                .unwrap()
                .remove(&flist_job_id.clone());

            return Err(ResponseError::InternalServerError);
        }
    }
}

#[utoipa::path(
	get,
	path = "/v1/api/fl",
	responses(
        (status = 200, description = "Listing flists", body = HashMap<String, Vec<FileInfo>>),
        (status = 401, description = "Unauthorized user"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal server error"),
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
                            return Err(ResponseError::InternalServerError);
                        }
                    };
                };
            }
        }
        Err(e) => {
            log::error!("failed to list flists directory with error: {}", e);
            return Err(ResponseError::InternalServerError);
        }
    }

    Ok(ResponseResult::Flists(flists))
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