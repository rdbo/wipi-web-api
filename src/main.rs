mod api;
mod service;

use std::sync::Arc;

use argon2::password_hash::PasswordHashString;
use axum::{Router, routing::post};
use chrono::Duration;

use crate::service::AuthService;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
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

    axum::serve(listener, app).await.unwrap();
}
