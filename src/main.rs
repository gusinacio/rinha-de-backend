use axum::Router;
use database::{migrate, Database};
use router::client_router;
use tokio::net::TcpListener;

mod database;
mod error;
mod router;
mod validator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database = Database::new();
    migrate(database.clone()).await;
    let clients = client_router();
    let app = Router::new()
        .nest("/clientes", clients)
        .with_state(database);

    let listener = TcpListener::bind("0.0.0.0:9999").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
