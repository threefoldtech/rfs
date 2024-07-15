mod config;
mod handler;

use anyhow::{Context, Result};
use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    BoxError, Router,
};
use clap::{ArgAction, Parser};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{runtime::Builder, signal};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tower_http::{add_extension::AddExtensionLayer, cors::CorsLayer};

use crate::handler::{
    create_flist_handler, get_flist_state_handler, get_flists, health_check_handler,
};

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
    let config = config::parse_config(&opts.config_path).context("failed to parse config file")?;

    // Set up application state for use with with_state().
    let jobs_state = Mutex::new(HashMap::new());
    let app_state = Arc::new(config::AppState { jobs_state });

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

    let cors = CorsLayer::new();
    // TODO:
    // .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
    // .allow_methods([Method::GET, Method::POST])
    // .allow_credentials(true)
    // .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    // TODO: add pagination
    let app = Router::new()
        .route(
            &format!("/{}/api", config.version),
            get(health_check_handler),
        )
        .route(
            &format!("/{}/api/fl", config.version),
            post(create_flist_handler),
        )
        .route(
            &format!("/{}/api/fl/:job_id", config.version),
            get(get_flist_state_handler),
        )
        .route(&format!("/{}/*path", config.flist_dir), get(get_flists))
        .layer(
            ServiceBuilder::new()
                // Handle errors from middleware
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

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

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
