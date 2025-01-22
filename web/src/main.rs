use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Datelike, Local, Utc};
use env::load_database_configuration;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

mod env;

struct AppState {
    db_pool: Pool<Postgres>,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // initialize tracing
    tracing_subscriber::fmt::init();
    println!("Starting application");

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let app = build_app().await;
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

pub async fn build_app() -> Router {
    // Create connection pool to DB and run migrations
    let database_config = load_database_configuration().expect("Unable to load DB info");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_config.url().as_str())
        .await
        .expect("Unable to create connection pool to DB");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Unable to run migrations against DB");
    println!("DB migrations done");

    // build our application with a route
    let app_state = Arc::new(AppState { db_pool: pool });
    Router::new()
        .route("/", get(root))
        .route("/measure", post(log_measure))
        .route("/now", get(get_now))
        .with_state(app_state)
}

// basic handler that responds with JSON payload
async fn root() -> (StatusCode, Json<ApiResponse>) {
    let response = ApiResponse { datetime: 47 };
    (StatusCode::OK, Json(response))
}

#[derive(Serialize)]
struct NowResponse {
    now: DateTime<Utc>,
    weekday: u8,
}

async fn get_now() -> Json<NowResponse> {
    let now: DateTime<Utc> = Utc::now();
    Json(NowResponse {
        now,
        weekday: now.weekday() as u8,
    })
}

async fn log_measure(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Measure>,
) -> StatusCode {
    // insert your application logic here
    println!(
        "{} ({}): T = {}, humidity = {}",
        payload.timestamp.with_timezone(&Local),
        payload.capteur_id,
        payload.temperature,
        payload.humidity
    );
    match sqlx::query(
        "INSERT INTO t_measures (timestamp, capteur, temperature, humidity) VALUES ($1, $2, $3, $4)",
    )
    .bind(payload.timestamp)
    .bind(payload.capteur_id)
    .bind(payload.temperature)
    .bind(payload.humidity)
    .execute(&state.db_pool)
    .await
    {
        Ok(_) => StatusCode::CREATED,
        Err(_) => StatusCode::BAD_REQUEST,
    }
}

#[derive(Deserialize)]
struct Measure {
    timestamp: chrono::DateTime<Utc>,
    humidity: f64,
    temperature: f64,
    capteur_id: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct ApiResponse {
    datetime: u8,
}

#[cfg(test)]
mod tests {
    use crate::build_app;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_log_measure() {
        let app = build_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/measure")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(String::from(
                        r#"
                    {
                    "timestamp": "2025-01-22T18:07:55+0000",
                    "capteur_id": "test",
                    "temperature": 12,
                    "humidity": 87
                    }
                    "#,
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }
}
