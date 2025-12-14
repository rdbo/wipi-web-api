use std::sync::{Arc, Mutex};

use axum::{Router, extract::State, response::IntoResponse, routing::post};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};
use uuid::{Timestamp, Uuid};

async fn handler(State(SessionState { session }): State<SessionState>) -> impl IntoResponse {
    let mut session = session.lock().unwrap();
    let session_id = Uuid::now_v7();
    let session_timestamp = session_id
        .get_timestamp()
        .expect("UUIDv7 must have a timestamp");
    let now = {
        let (secs, _) = session_timestamp.to_unix();
        OffsetDateTime::from_unix_timestamp(secs as i64).expect("failed to convert timestamp")
    };
    let expiration = now + Duration::minutes(15);
    *session = Some(Session {
        id: session_id.clone(),
        creation_datetime: now,
        expiration_datetime: expiration,
    });

    return session_id.to_string();
}

struct Session {
    id: Uuid,
    creation_datetime: OffsetDateTime,
    expiration_datetime: OffsetDateTime,
}

#[derive(Clone)]
struct SessionState {
    session: Arc<Mutex<Option<Session>>>,
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let api = Router::new().route("/login", post(handler));
    let app = Router::new().nest("/api", api).with_state(SessionState {
        session: Arc::new(Mutex::new(None)),
    });

    let hostaddr = "127.0.0.1:8080";
    let listener = tokio::net::TcpListener::bind(hostaddr)
        .await
        .expect(&format!("failed to bind to address '{}'", hostaddr));
    log::info!("Started listener at '{}'", hostaddr);

    axum::serve(listener, app).await.unwrap();
}
