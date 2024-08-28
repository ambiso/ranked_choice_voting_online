use std::sync::Arc;

use anyhow::Context;
use axum::{
    routing::{get, post},
    Extension, Router,
};

use clap::Parser;
use config::Config;
use http::add_candidate;
use sqlx::postgres::PgPoolOptions;

mod config;
mod error;

mod http;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    env_logger::init();

    let config = Config::parse();
    let db = PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database_url)
        .await
        .context("could not connect to database_url")?;

    // This embeds database migrations in the application binary so we can ensure the database
    // is migrated correctly on startup
    sqlx::migrate!().run(&db).await?;

    let app = Router::new()
        .route("/", get(http::index))
        .route("/election/create", post(http::create))
        .nest(
            "/election/:election_id",
            Router::new()
                .route("/", get(http::election))
                .route("/vote", post(http::vote))
                .route("/add_candidate", post(add_candidate)),
        )
        .layer(Extension(http::ApiContext {
            config: Arc::new(config),
            db,
        }));

    let listener = tokio::net::TcpListener::bind(":::3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
