mod api;
mod error;
mod extractor;
mod service;

use std::{net::SocketAddr, sync::Arc};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use argon2::password_hash::PasswordHashString;
use axum::{Router, routing::post};
use chrono::Duration;

use crate::service::AuthService;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_file(true)
        .with_line_number(true)
        .init();

    let auth_service = AuthService::new(
        PasswordHashString::new(
            "$argon2id$v=19$m=16,t=2,p=1$VnExMnQ0VWowbG5jc1NIcQ$mgaySsRJLlCOMzQymUBRzQ",
        )
        .expect("failed to parse argon2id hash"),
        Duration::seconds(15),
    );

    let api = Router::new().route("/login", post(api::login::post));
    let app = Router::new()
        .nest("/api", api)
        .with_state(Arc::new(auth_service));
    let hostaddr = "127.0.0.1:8080";
    let listener = tokio::net::TcpListener::bind(hostaddr)
        .await
        .unwrap_or_else(|_| panic!("failed to bind to address '{}'", hostaddr));
    log::info!("Started listener at '{}'", hostaddr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
