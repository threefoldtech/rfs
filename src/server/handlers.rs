use anyhow::Error;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use axum_macros::debug_handler;
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{mpsc, Arc},
};

use bollard::auth::DockerCredentials;
use serde::{Deserialize, Serialize};

use crate::docker;
use crate::fungi;
use crate::server::{
    auth::{SignInBody, SignInResponse, __path_sign_in_handler},
    config::{self, Job},
    db::DB,
    response::{DirListTemplate, DirLister, ErrorTemplate, TemplateErr},
    response::{FileInfo, ResponseError, ResponseResult, FlistStateResponse},
    serve_flists::visit_dir_one_level,
};
use crate::store;
use utoipa::{OpenApi, ToSchema, Modify};
use utoipa::openapi::security::{SecurityScheme, HttpAuthScheme, Http};
use uuid::Uuid;
use crate::server::block_handlers;
use crate::server::file_handlers;
use crate::server::serve_flists;
use crate::server::website_handlers;

// Security scheme modifier for JWT Bearer authentication
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap(); // Safe to unwrap since components are registered
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(health_check_handler, create_flist_handler, get_flist_state_handler, preview_flist_handler, list_flists_handler, sign_in_handler, block_handlers::upload_block_handler, block_handlers::get_block_handler, block_handlers::check_block_handler, block_handlers::verify_blocks_handler, block_handlers::get_blocks_by_hash_handler, block_handlers::list_blocks_handler, block_handlers::get_block_downloads_handler, block_handlers::get_user_blocks_handler, file_handlers::upload_file_handler, file_handlers::get_file_handler, website_handlers::serve_website_handler, serve_flists::serve_flists),
    modifiers(&SecurityAddon),
    components(
        schemas(
            // Common schemas
            DirListTemplate, DirLister, ResponseError, ErrorTemplate, TemplateErr, ResponseResult, FileInfo, FlistStateResponse,
            // Response wrapper schemas
            crate::server::response::HealthResponse, crate::server::response::BlockUploadedResponse,
            // Authentication schemas
            SignInBody, SignInResponse,
            // Flist schemas
            FlistBody, Job, FlistState, FlistStateInfo, PreviewResponse,
            // Block schemas
            crate::server::models::Block, block_handlers::VerifyBlock, block_handlers::VerifyBlocksRequest, block_handlers::VerifyBlocksResponse,
            block_handlers::BlocksResponse, block_handlers::ListBlocksParams, block_handlers::ListBlocksResponse, block_handlers::BlockInfo,
            block_handlers::UserBlocksResponse, block_handlers::BlockDownloadsResponse, block_handlers::UploadBlockParams,
            // File schemas
            file_handlers::FileUploadResponse, file_handlers::FileDownloadRequest, crate::server::models::File
        )
    ),
    tags(
        (name = "System", description = "System health and status"),
        (name = "Authentication", description = "Authentication endpoints"),
        (name = "Flist Management", description = "Flist creation and management"),
        (name = "Flist Serving", description = "Serving flist files"),
        (name = "Block Management", description = "Block storage and retrieval"),
        (name = "File Management", description = "File upload and download"),
        (name = "Website Serving", description = "Website content serving")
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

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct PreviewResponse {
    pub content: Vec<String>,
    pub metadata: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, ToSchema)]
pub enum FlistState {
    #[schema(title = "FlistStateAccepted")]
    Accepted(String),
    #[schema(title = "FlistStateStarted")]
    Started(String),
    #[schema(title = "FlistStateInProgress")]
    InProgress(FlistStateInfo),
    #[schema(title = "FlistStateCreated")]
    Created(String),
    #[schema(title = "FlistStateFailed")]
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, ToSchema)]
pub struct FlistStateInfo {
    msg: String,
    progress: f32,
}

#[utoipa::path(
    get,
    path = "/api/v1",
    tag = "System",
    responses(
        (status = 200, description = "flist server is working", body = HealthResponse)
    )
)]
pub async fn health_check_handler() -> ResponseResult {
    ResponseResult::Health
}

#[utoipa::path(
    post,
    path = "/api/v1/fl",
    tag = "Flist Management",
    request_body = FlistBody,
    responses(
        (status = 201, description = "Flist conversion started", body = Job),
        (status = 401, description = "Unauthorized user", body = ResponseError),
        (status = 403, description = "Forbidden", body = ResponseError),
        (status = 409, description = "Conflict", body = ResponseError),
        (status = 500, description = "Internal server error", body = ResponseError),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
#[debug_handler]
pub async fn create_flist_handler(
    State(state): State<Arc<config::AppState>>,
    Extension(username): Extension<String>,
    Json(body): Json<FlistBody>,
) -> impl IntoResponse {
    let cfg = state.config.clone();
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
    let username_dir = std::path::Path::new(&cfg.flist_dir).join(&username);
    let fl_path = username_dir.join(&fl_name);

    if fl_path.exists() {
        return Err(ResponseError::Conflict("flist already exists".to_string()));
    }

    if let Err(err) = fs::create_dir_all(&username_dir) {
        log::error!(
            "failed to create user flist directory `{:?}` with error {:?}",
            &username_dir,
            err
        );
        return Err(ResponseError::InternalServerError);
    }

    let meta = match fungi::Writer::new(&fl_path, true).await {
        Ok(writer) => writer,
        Err(err) => {
            log::error!(
                "failed to create a new writer for flist `{:?}` with error {}",
                fl_path,
                err
            );
            return Err(ResponseError::InternalServerError);
        }
    };

    let store = match store::parse_router(&cfg.store_url).await {
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

    state
        .jobs_state
        .lock()
        .expect("failed to lock state")
        .insert(
            job.id.clone(),
            FlistState::Accepted(format!("flist '{}' is accepted", &fl_name)),
        );

    let flist_download_url = std::path::Path::new(&format!("{}:{}", cfg.host, cfg.port))
        .join(cfg.flist_dir)
        .join(username)
        .join(&fl_name);

    tokio::spawn(async move {
        state
            .jobs_state
            .lock()
            .expect("failed to lock state")
            .insert(
                job.id.clone(),
                FlistState::Started(format!("flist '{}' is started", fl_name)),
            );

        let container_name = Uuid::new_v4().to_string();
        let docker_tmp_dir =
            tempdir::TempDir::new(&container_name).expect("failed to create tmp dir for docker");

        let (tx, rx) = mpsc::channel();
        let mut docker_to_fl =
            docker::DockerImageToFlist::new(meta, docker_image, credentials, docker_tmp_dir);

        let res = docker_to_fl.prepare().await;
        if res.is_err() {
            let _ = tokio::fs::remove_file(&fl_path).await;
            state
                .jobs_state
                .lock()
                .expect("failed to lock state")
                .insert(job.id.clone(), FlistState::Failed);
            return;
        }

        let files_count = docker_to_fl.files_count();
        let st = state.clone();
        let job_id = job.id.clone();
        let cloned_fl_path = fl_path.clone();
        tokio::spawn(async move {
            let mut progress: f32 = 0.0;

            for _ in 0..files_count - 1 {
                let step = rx.recv().expect("failed to receive progress") as f32;
                progress += step;
                let progress_percentage = progress / files_count as f32 * 100.0;
                st.jobs_state.lock().expect("failed to lock state").insert(
                    job_id.clone(),
                    FlistState::InProgress(FlistStateInfo {
                        msg: "flist is in progress".to_string(),
                        progress: progress_percentage,
                    }),
                );
                st.flists_progress
                    .lock()
                    .expect("failed to lock state")
                    .insert(cloned_fl_path.clone(), progress_percentage);
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
                .expect("failed to lock state")
                .insert(job.id.clone(), FlistState::Failed);
            return;
        }

        state
            .jobs_state
            .lock()
            .expect("failed to lock state")
            .insert(
                job.id.clone(),
                FlistState::Created(format!(
                    "flist {:?} is created successfully",
                    flist_download_url
                )),
            );
        state
            .flists_progress
            .lock()
            .expect("failed to lock state")
            .insert(fl_path, 100.0);
    });

    Ok(ResponseResult::FlistCreated(current_job))
}

#[utoipa::path(
    get,
    path = "/api/v1/fl/{job_id}",
    tag = "Flist Management",
    responses(
        (status = 200, description = "Flist state", body = FlistStateResponse),
        (status = 404, description = "Flist not found", body = ResponseError),
        (status = 500, description = "Internal server error", body = ResponseError),
        (status = 401, description = "Unauthorized user", body = ResponseError),
        (status = 403, description = "Forbidden", body = ResponseError),
    ),
    params(
        ("job_id" = String, Path, description = "flist job id")
    ),
    security(
        ("bearerAuth" = [])
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
        .expect("failed to lock state")
        .contains_key(&flist_job_id.clone())
    {
        return Err(ResponseError::NotFound("flist doesn't exist".to_string()));
    }

    let res_state = state
        .jobs_state
        .lock()
        .expect("failed to lock state")
        .get(&flist_job_id.clone())
        .expect("failed to get from state")
        .to_owned();

    match res_state {
        FlistState::Accepted(_) => Ok(ResponseResult::FlistState(res_state)),
        FlistState::Started(_) => Ok(ResponseResult::FlistState(res_state)),
        FlistState::InProgress(_) => Ok(ResponseResult::FlistState(res_state)),
        FlistState::Created(_) => {
            state
                .jobs_state
                .lock()
                .expect("failed to lock state")
                .remove(&flist_job_id.clone());

            Ok(ResponseResult::FlistState(res_state))
        }
        FlistState::Failed => {
            state
                .jobs_state
                .lock()
                .expect("failed to lock state")
                .remove(&flist_job_id.clone());

            Err(ResponseError::InternalServerError)
        }
    }
}

#[utoipa::path(
	get,
	path = "/api/v1/fl",
	tag = "Flist Management",
	responses(
        (status = 200, description = "Listing flists", body = HashMap<String, Vec<FileInfo>>),
        (status = 401, description = "Unauthorized user", body = ResponseError),
        (status = 403, description = "Forbidden", body = ResponseError),
        (status = 500, description = "Internal server error", body = ResponseError),
	)
)]
#[debug_handler]
pub async fn list_flists_handler(State(state): State<Arc<config::AppState>>) -> impl IntoResponse {
    let mut flists: HashMap<String, Vec<FileInfo>> = HashMap::new();

    let rs: Result<Vec<FileInfo>, std::io::Error> =
        visit_dir_one_level(&state.config.flist_dir, &state).await;

    let files = match rs {
        Ok(files) => files,
        Err(e) => {
            log::error!("failed to list flists directory with error: {}", e);
            return Err(ResponseError::InternalServerError);
        }
    };

    for file in files {
        if !file.is_file {
            let flists_per_username = visit_dir_one_level(&file.path_uri, &state).await;
            match flists_per_username {
                Ok(files) => flists.insert(file.name, files),
                Err(e) => {
                    log::error!("failed to list flists per username with error: {}", e);
                    return Err(ResponseError::InternalServerError);
                }
            };
        };
    }

    Ok(ResponseResult::Flists(flists))
}

#[utoipa::path(
	get,
	path = "/api/v1/fl/preview/{flist_path}",
	tag = "Flist Management",
	responses(
        (status = 200, description = "Flist preview result", body = PreviewResponse),
        (status = 400, description = "Bad request", body = ResponseError),
        (status = 401, description = "Unauthorized user", body = ResponseError),
        (status = 403, description = "Forbidden", body = ResponseError),
        (status = 500, description = "Internal server error", body = ResponseError),
	),
    params(
        ("flist_path" = String, Path, description = "flist file path")
    )
)]
#[debug_handler]
pub async fn preview_flist_handler(
    State(state): State<Arc<config::AppState>>,
    Path(flist_path): Path<String>,
) -> impl IntoResponse {
    let fl_path = flist_path;

    match validate_flist_path(&state, &fl_path).await {
        Ok(_) => (),
        Err(err) => return Err(ResponseError::BadRequest(err.to_string())),
    };

    let content = match get_flist_content(&fl_path).await {
        Ok(paths) => paths,
        Err(_) => return Err(ResponseError::InternalServerError),
    };

    let bytes = match std::fs::read(&fl_path) {
        Ok(b) => b,
        Err(err) => {
            log::error!(
                "failed to read flist '{}' into bytes with error {}",
                fl_path,
                err
            );
            return Err(ResponseError::InternalServerError);
        }
    };

    // Convert PathBuf values to strings for OpenAPI compatibility
    let content_strings: Vec<String> = content
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect();
        
    Ok(ResponseResult::PreviewFlist(PreviewResponse {
        content: content_strings,
        metadata: state.config.store_url.join("-"),
        checksum: sha256::digest(&bytes),
    }))
}

async fn validate_flist_path(state: &Arc<config::AppState>, fl_path: &String) -> Result<(), Error> {
    // validate path starting with `/`
    if fl_path.starts_with("/") {
        anyhow::bail!("invalid flist path '{}', shouldn't start with '/'", fl_path);
    }

    // path should include 3 parts [parent dir, username, flist file]
    let parts: Vec<_> = fl_path.split("/").collect();
    if parts.len() != 3 {
        anyhow::bail!(
            format!("invalid flist path '{}', should consist of 3 parts [parent directory, username and flist name", fl_path
        ));
    }

    // validate parent dir
    if parts[0] != state.config.flist_dir {
        anyhow::bail!(
            "invalid flist path '{}', parent directory should be '{}'",
            fl_path,
            state.config.flist_dir
        );
    }

    // validate username
    match state.db.get_user_by_username(parts[1]).await {
        Some(_) => (),
        None => {
            anyhow::bail!(
                "invalid flist path '{}', username '{}' doesn't exist",
                fl_path,
                parts[1]
            );
        }
    };

    // validate flist extension
    let fl_name = parts[2].to_string();
    let ext = match std::path::Path::new(&fl_name).extension() {
        Some(ex) => ex.to_string_lossy().to_string(),
        None => "".to_string(),
    };

    if ext != "fl" {
        anyhow::bail!(
            "invalid flist path '{}', invalid flist extension '{}' should be 'fl'",
            fl_path,
            ext
        );
    }

    // validate flist existence
    if !std::path::Path::new(parts[0])
        .join(parts[1])
        .join(&fl_name)
        .exists()
    {
        anyhow::bail!("flist '{}' doesn't exist", fl_path);
    }

    Ok(())
}

async fn get_flist_content(fl_path: &String) -> Result<Vec<PathBuf>, Error> {
    let mut visitor = ReadVisitor::default();

    let meta = match fungi::Reader::new(&fl_path).await {
        Ok(reader) => reader,
        Err(err) => {
            log::error!(
                "failed to initialize metadata database for flist `{}` with error {}",
                fl_path,
                err
            );
            anyhow::bail!("Internal server error");
        }
    };

    match meta.walk(&mut visitor).await {
        Ok(()) => return Ok(visitor.into_inner()),
        Err(err) => {
            log::error!(
                "failed to walk through metadata for flist `{}` with error {}",
                fl_path,
                err
            );
            anyhow::bail!("Internal server error");
        }
    };
}

#[derive(Default)]
struct ReadVisitor {
    inner: Vec<PathBuf>,
}

impl ReadVisitor {
    pub fn into_inner(self) -> Vec<PathBuf> {
        self.inner
    }
}

#[async_trait::async_trait]
impl fungi::meta::WalkVisitor for ReadVisitor {
    async fn visit(
        &mut self,
        path: &std::path::Path,
        _node: &fungi::meta::Inode,
    ) -> fungi::meta::Result<fungi::meta::Walk> {
        self.inner.push(path.to_path_buf());
        Ok(fungi::meta::Walk::Continue)
    }
}
