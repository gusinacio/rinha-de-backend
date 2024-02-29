use redis::{AsyncCommands, Client as RedisClient};

use crate::{
    database::{Balance, PostgresDatabase, Statement, Transaction, TransactionRepository},
    error::ServerError,
};

#[derive(Clone)]
pub struct CachedDatabase<T>
where
    T: TransactionRepository,
{
    cache: RedisClient,
    database: T,
}

impl<T> CachedDatabase<T>
where
    T: TransactionRepository,
{
    pub fn new(cache: RedisClient, database: T) -> Self {
        Self { cache, database }
    }
}

impl TransactionRepository for CachedDatabase<PostgresDatabase> {
    async fn add_transaction(
        &self,
        id: u32,
        transaction: Transaction,
    ) -> Result<Balance, ServerError> {
        let mut connection = self.cache.get_async_connection().await?;
        let (total, limit): (Option<i32>, Option<i32>) = connection
            .mget(vec![
                format!("balance:{id}:total"),
                format!("balance:{id}:limit"),
            ])
            .await?;

        // early check if the transaction would exceed the limit
        let tx_value = transaction.value();
        if let (Some(total), Some(limit)) = (total, limit) {
            if total + tx_value < -limit {
                return Err(ServerError::TransactionWouldExceedLimit);
            }
        }
        let balance = self.database.add_transaction(id, transaction).await?;

        // update the cache
        // set limit
        if let None = limit {
            connection
                .set(format!("balance:{id}:limit"), balance.limit)
                .await?;
        }

        match total {
            // something is wrong, update the cache
            Some(total) if total + tx_value != balance.total => {
                println!("Cache is out of sync, updating");
                connection
                    .set(format!("balance:{id}:total"), balance.total)
                    .await?
            }
            _ => {
                connection
                    .incr(format!("balance:{id}:total"), tx_value)
                    .await?
            }
        }

        Ok(balance)
    }

    async fn get_statement(&self, id: &u32) -> Result<Statement, ServerError> {
        let stmt = self.database.get_statement(id).await?;
        Ok(stmt)
    }
}
