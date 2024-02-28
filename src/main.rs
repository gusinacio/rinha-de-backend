use axum::Router;
use database::Database;
use router::client_router;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

mod database;
mod error;
mod router;
mod validator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let database_connections = std::env::var("DATABASE_CONNECTIONS")
        .unwrap_or("10".to_string())
        .parse()
        .expect("DATABASE_CONNECTIONS must be a number");

    let pool = PgPoolOptions::new()
        .max_connections(database_connections)
        .connect(&database_url)
        .await?;
    let database = Database::new(pool);
    let clients = client_router();
    let app = Router::new()
        .nest("/clientes", clients)
        .with_state(database);

    let port = std::env::var("PORT").unwrap_or("9999".to_string());

    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
