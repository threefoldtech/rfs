use axum::extract::State;
use std::{path::PathBuf, sync::Arc};
use tokio::io;
use tower::util::ServiceExt;
use tower_http::services::ServeDir;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use axum_macros::debug_handler;
use percent_encoding::percent_decode;

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

pub async fn visit_dir_one_level<P: AsRef<std::path::Path>>(
    path: P,
    state: &Arc<config::AppState>,
) -> io::Result<Vec<FileInfo>> {
    let path = path.as_ref();
    let mut dir = tokio::fs::read_dir(path).await?;
    let mut files: Vec<FileInfo> = Vec::new();

    while let Some(child) = dir.next_entry().await? {
        let path_uri = child.path().to_string_lossy().to_string();
        let is_file = child.file_type().await?.is_file();
        let name = child.file_name().to_string_lossy().to_string();

        let mut progress = 0.0;
        if is_file {
            match state.flists_progress.lock().unwrap().get(&path.join(&name)) {
                Some(p) => progress = *p,
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
