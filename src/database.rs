pub use cache::CachedDatabase;
pub use models::*;
pub use mongo::MongoDatabase;
pub use postgres::PostgresDatabase;

use crate::error::ServerError;

mod cache;
mod models;
mod postgres;
mod mongo;

#[derive(Clone)]
pub enum Database {
    Postgres(PostgresDatabase),
    Mongo(MongoDatabase),
    Cached(CachedDatabase<PostgresDatabase>),
}

impl TransactionRepository for Database {
    async fn add_transaction(
        &self,
        id: u32,
        transaction: Transaction,
    ) -> Result<Balance, ServerError> {
        match self {
            Database::Postgres(database) => database.add_transaction(id, transaction).await,
            Database::Cached(database) => database.add_transaction(id, transaction).await,
            Database::Mongo(database) => database.add_transaction(id, transaction).await,
        }
    }


    async fn get_statement(&self, id: &u32) -> Result<Statement, ServerError> {
        match self {
            Database::Postgres(database) => database.get_statement(id).await,
            Database::Cached(database) => database.get_statement(id).await,
            Database::Mongo(database) => database.get_statement(id).await,
        }
    }
}

pub trait TransactionRepository {
    async fn add_transaction(
        &self,
        id: u32,
        transaction: Transaction,
    ) -> Result<Balance, ServerError>;
    async fn get_statement(&self, id: &u32) -> Result<Statement, ServerError>;
}

#[cfg(test)]
mod tests {
    use crate::router::NewTransaction;

    use super::*;

    #[test]
    fn should_not_exceed_limit() {
        // let mut stmt = Statement::new(1000);
        // let transaction =
        NewTransaction::new(1001, TransactionType::Withdraw, "description".to_string());
        // let result = stmt.add_transaction(transaction.into());
        // assert!(result.is_err());
        //
        // let transaction =
        //     NewTransaction::new(1000, TransactionType::Withdraw, "description".to_string());
        // let result = stmt.add_transaction(transaction.into());
        // assert!(result.is_ok());
    }
}
