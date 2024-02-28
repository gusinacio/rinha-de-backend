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
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect("postgres://postgres:postgres@localhost:5434/postgres")
        .await?;
    let database = Database::new(pool);
    let clients = client_router();
    let app = Router::new()
        .nest("/clientes", clients)
        .with_state(database);

    let listener = TcpListener::bind("0.0.0.0:9999").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
