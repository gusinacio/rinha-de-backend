use crate::error::ServerError;

mod cache;
mod models;
mod postgres;

pub use models::*;
pub use postgres::PostgresDatabase;
pub use cache::CachedDatabase;

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
    use super::*;
    use crate::router::NewTransaction;

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
