use axum::Router;
use redis::Client;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

use database::CachedDatabase;
use database::PostgresDatabase;
use router::client_router;

use crate::database::MongoDatabase;

mod database;
mod error;
mod router;
mod validator;

type Database = database::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_type = std::env::var("DATABASE_TYPE").unwrap_or("postgres".to_string());

    let database = match database_type.as_str() {
        "postgres" => {
            let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
            let database_connections = std::env::var("DATABASE_CONNECTIONS")
                .unwrap_or("10".to_string())
                .parse()
                .expect("DATABASE_CONNECTIONS must be a number");

            let pool = PgPoolOptions::new()
                .max_connections(database_connections)
                .connect(&database_url)
                .await?;
            let postgres_db = PostgresDatabase::new(pool);

            let redis_url = std::env::var("REDIS_URL").ok();
            match redis_url {
                Some(redis_url) => {
                    let redis = Client::open(redis_url)?;
                    let cached_database = CachedDatabase::new(redis, postgres_db);
                    Database::Cached(cached_database)
                }
                None => Database::Postgres(postgres_db),
            }
        }
        "mongo" => {
            let database_url = std::env::var("MONGO_URL").expect("MONGO_URL must be set");
            let client = mongodb::Client::with_uri_str(&database_url).await?;
            let mongo = MongoDatabase::new(client).await;
            Database::Mongo(mongo)
        }
        _ => panic!("DATABASE_TYPE must be a postgres or mongo"),
    };

    let clients = client_router();
    let app = Router::new()
        .nest("/clientes", clients)
        .with_state(database);

    let port = std::env::var("PORT").unwrap_or("9999".to_string());

    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
