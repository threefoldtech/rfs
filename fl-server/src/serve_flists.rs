use axum::extract::State;
use std::{path::PathBuf, sync::Arc};
use tokio::io;
use tower::util::ServiceExt;
use tower_http::services::ServeDir;
use walkdir::WalkDir;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use axum_macros::debug_handler;
use percent_encoding::percent_decode;
use rfs::{cache, fungi::Reader};

use crate::{
    config,
    response::{
        DirListTemplate, DirLister, ErrorTemplate, FileInfo, ResponseError, ResponseResult,
        TemplateErr,
    },
};

#[debug_handler]
pub async fn serve_flists(
    State(state): State<Arc<config::AppState>>,
    req: Request<Body>,
) -> impl IntoResponse {
    let path = req.uri().path().to_string();

    if path.ends_with(".md") {
        match preview_flist(&path).await {
            Ok(res) => return Ok(res),
            Err(err) => return Err(err),
        };
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
                            return Err(ResponseError::TemplateError(ErrorTemplate {
                                err: TemplateErr::BadRequest("invalid path".to_string()),
                                cur_path: path.to_string(),
                                message: "invalid path".to_owned(),
                            }));
                        }
                        full_path.push(seg);
                    }

                    let cur_path = std::path::Path::new(&full_path);

                    match cur_path.is_dir() {
                        true => {
                            let rs = visit_dir_one_level(&full_path, &state).await;
                            match rs {
                                Ok(files) => Ok(ResponseResult::DirTemplate(DirListTemplate {
                                    lister: DirLister { files },
                                    cur_path: path.to_string(),
                                })),
                                Err(e) => Err(ResponseError::TemplateError(ErrorTemplate {
                                    err: TemplateErr::InternalServerError(e.to_string()),
                                    cur_path: path.to_string(),
                                    message: e.to_string(),
                                })),
                            }
                        }
                        false => Err(ResponseError::TemplateError(ErrorTemplate {
                            err: TemplateErr::NotFound("file not found".to_string()),
                            cur_path: path.to_string(),
                            message: "file not found".to_owned(),
                        })),
                    }
                }
                _ => Ok(ResponseResult::Res(res)),
            }
        }
        Err(err) => Err(ResponseError::TemplateError(ErrorTemplate {
            err: TemplateErr::InternalServerError(format!("Unhandled error: {}", err)),
            cur_path: path.to_string(),
            message: format!("Unhandled error: {}", err),
        })),
    };
}

pub async fn visit_dir_one_level(
    path: &std::path::Path,
    state: &Arc<config::AppState>,
) -> io::Result<Vec<FileInfo>> {
    let mut dir = tokio::fs::read_dir(path).await?;
    let mut files: Vec<FileInfo> = Vec::new();

    while let Some(child) = dir.next_entry().await? {
        let path_uri = child.path().to_string_lossy().to_string();
        let is_file = child.file_type().await?.is_file();
        let name = child.file_name().to_string_lossy().to_string();

        let mut progress = 0.0;
        if is_file {
            match state.flists_progress.lock().unwrap().get(&format!(
                "{}/{}",
                path.to_string_lossy().to_string(),
                name
            )) {
                Some(p) => progress = p.to_owned(),
                None => progress = 100.0,
            }

            let ext = child
                .path()
                .extension()
                .unwrap()
                .to_string_lossy()
                .to_string();
            if ext != "fl" {
                continue;
            }
        }

        files.push(FileInfo {
            name,
            path_uri,
            is_file,
            size: child.metadata().await?.len(),
            last_modified: child
                .metadata()
                .await?
                .modified()?
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            progress,
        });
    }

    Ok(files)
}

async fn preview_flist(path: &String) -> Result<ResponseResult, ResponseError> {
    if !path.ends_with(".md") {
        return Err(ResponseError::BadRequest(
            "flist path is invalid".to_string(),
        ));
    }

    let mut fl_path: String = path.strip_suffix(".md").unwrap().to_string();
    fl_path = fl_path.strip_prefix("/").unwrap().to_string();
    let meta = match Reader::new(&fl_path).await {
        Ok(reader) => reader,
        Err(err) => {
            log::error!(
                "failed to initialize metadata database for flist `{}` with error {}",
                fl_path,
                err
            );
            return Err(ResponseError::InternalServerError);
        }
    };

    let router = match rfs::store::get_router(&meta).await {
        Ok(r) => r,
        Err(err) => {
            log::error!("failed to get router with error {}", err);
            return Err(ResponseError::InternalServerError);
        }
    };

    let cache = cache::Cache::new(String::from("/tmp/cache"), router);
    let tmp_target = tempdir::TempDir::new("target").unwrap();
    let tmp_target_path = tmp_target.path().to_owned();

    match rfs::unpack(&meta, &cache, &tmp_target_path, false).await {
        Ok(_) => (),
        Err(err) => {
            log::error!("failed to unpack flist {} with error {}", fl_path, err);
            return Err(ResponseError::InternalServerError);
        }
    };

    let mut paths = Vec::new();
    for file in WalkDir::new(tmp_target_path.clone())
        .into_iter()
        .filter_map(|file| file.ok())
    {
        let mut path = file.path().to_string_lossy().to_string();
        path = path
            .strip_prefix(&tmp_target_path.to_string_lossy().to_string())
            .unwrap()
            .to_string();
        paths.push(path);
    }

    Ok(ResponseResult::PreviewFlist(paths))
}
