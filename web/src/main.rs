use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use env::load_database_configuration;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

mod env;
mod measure;
mod rtc;

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
    let pool = create_db_pool().await;

    // build our application with a route
    let app_state = Arc::new(AppState { db_pool: pool });
    Router::new()
        .route("/measure", post(measure::log_measure))
        .route("/now", get(rtc::get_now))
        .with_state(app_state)
}
async fn create_db_pool() -> Pool<Postgres> {
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

    pool
}
