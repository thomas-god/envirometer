use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Datelike, Local, Utc};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/measure", post(log_measure))
        .route("/now", get(get_now));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<Measure>,
) -> StatusCode {
    // insert your application logic here
    println!(
        "{}: T = {}, humidity = {}",
        payload.timestamp.with_timezone(&Local),
        payload.temperature,
        payload.humidity
    );

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    StatusCode::ACCEPTED
}

#[derive(Deserialize)]
struct Measure {
    timestamp: chrono::DateTime<Utc>,
    humidity: f64,
    temperature: f64,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct ApiResponse {
    datetime: u8,
}
