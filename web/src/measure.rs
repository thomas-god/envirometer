use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use chrono::{Local, Utc};
use serde::Deserialize;

use crate::AppState;

#[derive(Deserialize)]
pub struct Measure {
    timestamp: chrono::DateTime<Utc>,
    humidity: f64,
    temperature: f64,
    capteur_id: String,
}

pub async fn log_measure(
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

#[cfg(test)]
mod tests {
    use crate::{create_db_pool, AppState};
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
        routing::post,
        Router,
    };
    use std::env;
    use std::sync::Arc;
    use tower::ServiceExt;

    use super::log_measure;

    async fn build_test_app() -> Router {
        dotenvy::dotenv().expect("Failed to load .env");
        println!("{:?}", env::var("PSQL_PWD"));
        let db_pool = create_db_pool().await;
        let app_state = Arc::new(AppState { db_pool });
        Router::new()
            .route("/measure", post(log_measure))
            .with_state(app_state)
    }

    #[tokio::test]
    async fn test_log_measure() {
        let app = build_test_app().await;

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
