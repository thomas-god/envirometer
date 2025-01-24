use axum::Json;
use chrono::{DateTime, Datelike, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct NowResponse {
    now: DateTime<Utc>,
    weekday: u8,
}

pub async fn get_now() -> Json<NowResponse> {
    let now: DateTime<Utc> = Utc::now();
    Json(NowResponse {
        now,
        weekday: now.weekday() as u8,
    })
}
