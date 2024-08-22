mod auth;
mod config;
mod handlers;
mod response;
mod serve_flists;

use anyhow::{Context, Result};
use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{get, post},
    BoxError, Router,
};
use clap::{ArgAction, Parser};
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
use tokio::{runtime::Builder, signal};
use tower::ServiceBuilder;
use tower_http::{add_extension::AddExtensionLayer, cors::CorsLayer};
use tower_http::{cors::Any, trace::TraceLayer};

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Parser, Debug)]
#[clap(name ="fl-server", author, version = env!("GIT_VERSION"), about, long_about = None)]
struct Options {
    /// enable debugging logs
    #[clap(short, long, action=ArgAction::Count)]
    debug: u8,

    /// config file path
    #[clap(short, long)]
    config_path: String,
}

fn main() -> Result<()> {
    let rt = Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(app())
}

async fn app() -> Result<()> {
    let opts = Options::parse();
    simple_logger::SimpleLogger::new()
        .with_utc_timestamps()
        .with_level({
            match opts.debug {
                0 => log::LevelFilter::Info,
                1 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            }
        })
        .with_module_level("sqlx", log::Level::Error.to_level_filter())
        .init()?;

    let config = config::parse_config(&opts.config_path)
        .await
        .context("failed to parse config file")?;

    let app_state = Arc::new(config::AppState {
        jobs_state: Mutex::new(HashMap::new()),
        flists_progress: Mutex::new(HashMap::new()),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    let v1_routes = Router::new()
        .route("/v1/api", get(handlers::health_check_handler))
        .route("/v1/api/signin", post(auth::sign_in_handler))
        .route(
            "/v1/api/fl",
            post(handlers::create_flist_handler).layer(middleware::from_fn_with_state(
                config.clone(),
                auth::authorize,
            )),
        )
        .route(
            "/v1/api/fl/:job_id",
            get(handlers::get_flist_state_handler).layer(middleware::from_fn_with_state(
                config.clone(),
                auth::authorize,
            )),
        )
        .route(
            "/v1/api/fl/preview/:flist_path",
            get(handlers::preview_flist_handler),
        )
        .route("/v1/api/fl", get(handlers::list_flists_handler))
        .route("/*path", get(serve_flists::serve_flists));

    let app = Router::new()
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", handlers::FlistApi::openapi()),
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
        .layer(AddExtensionLayer::new(config.clone()))
        .with_state(Arc::clone(&app_state))
        .layer(cors);

    let address = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .context("failed to bind address")?;

    log::info!(
        "🚀 Server started successfully at {}:{}",
        config.host,
        config.port
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
