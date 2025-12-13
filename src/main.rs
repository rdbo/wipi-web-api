use axum::{Router, routing::post};

async fn handler() -> &'static str {
    return "OK";
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let app = Router::new().route("/api/login", post(handler));

    let hostaddr = "127.0.0.1:8080";
    let listener = tokio::net::TcpListener::bind(hostaddr)
        .await
        .expect(&format!("failed to bind to address '{}'", hostaddr));
    log::info!("Started listener at '{}'", hostaddr);

    axum::serve(listener, app).await.unwrap();
}
