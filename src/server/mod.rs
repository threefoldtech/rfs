mod auth;
mod block_handlers;
mod config;
mod db;
mod file_handlers;
mod handlers;
mod models;
mod response;
mod serve_flists;
mod website_handlers;

use anyhow::{Context, Result};
use axum::{
    error_handling::HandleErrorLayer,
    extract::{Path, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{get, head, post},
    BoxError, Router,
};
use config::AppState;
use hyper::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    Method,
};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::{cors::Any, trace::TraceLayer};

use block_handlers::BlockApi;
use file_handlers::FileApi;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use website_handlers::WebsiteApi;

pub async fn app(config_path: &str) -> Result<()> {
    let config = config::parse_config(config_path)
        .await
        .context("failed to parse config file")?;

    // Initialize the database based on configuration
    let db: Arc<db::DBType> = if let Some(sqlite_path) = &config.sqlite_path {
        log::info!("Using SQLite database at: {}", sqlite_path);
        Arc::new(db::DBType::SqlDB(
            db::sqlite::SqlDB::new(sqlite_path, &config.storage_dir, &config.users.clone()).await,
        ))
    } else {
        log::info!("Using in-memory MapDB database");
        Arc::new(db::DBType::MapDB(db::map::MapDB::new(
            &config.users.clone(),
        )))
    };

    let app_state = Arc::new(config::AppState {
        jobs_state: Mutex::new(HashMap::new()),
        flists_progress: Mutex::new(HashMap::new()),
        db,
        config,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    let v1_routes = Router::new()
        .route("/api/v1", get(handlers::health_check_handler))
        .route("/api/v1/signin", post(auth::sign_in_handler))
        .route(
            "/api/v1/fl",
            post(handlers::create_flist_handler).layer(middleware::from_fn_with_state(
                app_state.clone(),
                auth::authorize,
            )),
        )
        .route(
            "/api/v1/fl/:job_id",
            get(handlers::get_flist_state_handler).layer(middleware::from_fn_with_state(
                app_state.clone(),
                auth::authorize,
            )),
        )
        .route(
            "/api/v1/fl/preview/:flist_path",
            get(handlers::preview_flist_handler),
        )
        .route("/api/v1/fl", get(handlers::list_flists_handler))
        .route(
            "/api/v1/block",
            post(block_handlers::upload_block_handler).layer(middleware::from_fn_with_state(
                app_state.clone(),
                auth::authorize,
            )),
        )
        .route(
            "/api/v1/block/:hash",
            get(block_handlers::get_block_handler),
        )
        .route(
            "/api/v1/block/:hash",
            head(block_handlers::check_block_handler),
        )
        .route(
            "/api/v1/block/verify",
            post(block_handlers::verify_blocks_handler),
        )
        .route(
            "/api/v1/blocks/:hash",
            get(block_handlers::get_blocks_by_hash_handler),
        )
        .route("/api/v1/blocks", get(block_handlers::list_blocks_handler))
        .route(
            "/api/v1/block/:hash/downloads",
            get(block_handlers::get_block_downloads_handler),
        )
        .route(
            "/api/v1/user/blocks",
            get(block_handlers::get_user_blocks_handler).layer(middleware::from_fn_with_state(
                app_state.clone(),
                auth::authorize,
            )),
        )
        .route(
            "/api/v1/file",
            post(file_handlers::upload_file_handler).layer(middleware::from_fn_with_state(
                app_state.clone(),
                auth::authorize,
            )),
        )
        .route("/api/v1/file/:hash", get(file_handlers::get_file_handler))
        .route(
            "/website/:website_hash/*path",
            get(website_handlers::serve_website_handler),
        )
        .route(
            "/website/:website_hash/",
            get(
                |state: State<Arc<AppState>>, path: Path<String>| async move {
                    website_handlers::serve_website_handler(state, Path((path.0, "".to_string())))
                        .await
                },
            ),
        )
        .route("/*path", get(serve_flists::serve_flists));

    let app = Router::new()
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", handlers::FlistApi::openapi())
                .url("/api-docs/block-api.json", BlockApi::openapi())
                .url("/api-docs/file-api.json", FileApi::openapi())
                .url("/api-docs/website-api.json", WebsiteApi::openapi()),
        )
        .merge(v1_routes)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_error))
                .load_shed()
                .concurrency_limit(1024)
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http()),
        )
        .with_state(Arc::clone(&app_state))
        .layer(cors);

    let address = format!("{}:{}", app_state.config.host, app_state.config.port);
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .context("failed to bind address")?;

    log::info!(
        "🚀 Server started successfully at {}:{}",
        app_state.config.host,
        app_state.config.port
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("failed to serve listener")?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn handle_error(error: BoxError) -> impl IntoResponse {
    if error.is::<tower::timeout::error::Elapsed>() {
        return (StatusCode::REQUEST_TIMEOUT, Cow::from("request timed out"));
    }

    if error.is::<tower::load_shed::error::Overloaded>() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Cow::from("service is overloaded, try again later"),
        );
    }

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Cow::from(format!("Unhandled internal error: {}", error)),
    )
}
