mod api;
mod error;
mod extractor;
mod service;

use futures_util::stream::TryStreamExt;
use rtnetlink::packet_route::link::LinkAttribute;
use std::{net::SocketAddr, sync::Arc};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use argon2::password_hash::PasswordHashString;
use axum::{Extension, Router, routing::post};
use chrono::Duration;

use crate::service::{AuthService, NetlinkService};

pub struct AppState {}

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

    tracing::info!("Parsing configuration...");
    let admin_password_hash = PasswordHashString::new(
        "$argon2id$v=19$m=16,t=2,p=1$bmdTS2pGWGJVNmdORUo2ZQ$zA44lW9pmTUFzgRt1K8jPg",
    )
    .expect("failed to parse argon2id hash");

    tracing::info!("Initializing services...");
    let netlink_service = NetlinkService::try_new().expect("failed to initialize netlink service");
    let auth_service = AuthService::new(
        admin_password_hash,
        Duration::minutes(15),
        Duration::seconds(15),
    );

    tracing::info!("Setting up routes...");
    let net = Router::new().route("/interfaces", post(api::net::interfaces::post));
    let api = Router::new()
        .route("/login", post(api::login::post))
        .route("/logout", post(api::logout::post))
        .route("/auth_status", post(api::auth_status::post))
        .nest("/net", net);
    let app = Router::new()
        .nest("/api", api)
        .layer(Extension(Arc::new(auth_service)))
        .layer(Extension(Arc::new(netlink_service)));
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
