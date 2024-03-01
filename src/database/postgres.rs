use super::{Balance, Statement, Transaction, TransactionRepository, TransactionType};
use chrono::Utc;
use sqlx::{Pool, Postgres};

use crate::error::ServerError;

#[derive(Clone)]
pub struct PostgresDatabase(Pool<Postgres>);

impl TransactionRepository for PostgresDatabase {
    async fn add_transaction(
        &self,
        id: u32,
        transaction: Transaction,
    ) -> Result<Balance, ServerError> {
        let mut tx = self.0.begin().await?;
        let balance = sqlx::query!(
            "UPDATE wallet SET total = total + $2 WHERE id = $1 RETURNING *;",
            id as i32,
            transaction.value()
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) if db_err.code() == Some("23514".into()) => {
                ServerError::TransactionWouldExceedLimit
            }
            _ => e.into(),
        })?;
        let balance = Balance {
            total: balance.total,
            limit: balance.limit as u32,
            statement_date: Some(Utc::now()),
        };
        sqlx::query!(
            "INSERT INTO transaction (wallet_id, amount, \"type\", description) VALUES ($1, $2, $3, $4);",
            id as i32,
            transaction.value as i32,
            transaction.transaction_type as TransactionType,
            transaction.description
        ).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(balance)
    }

    async fn get_statement(&self, id: &u32) -> Result<Statement, ServerError> {
        let id = *id as i32;
        let balance = sqlx::query!("SELECT total, \"limit\" FROM wallet WHERE id = $1;", id)
            .fetch_one(&self.0)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => ServerError::UserNotFound(id as u32),
                _ => e.into(),
            })?;
        let balance = Balance {
            total: balance.total,
            limit: balance.limit as u32,
            statement_date: Some(Utc::now()),
        };
        let transactions = sqlx::query!(
            r#"SELECT 
                amount, 
                "type" as "type: TransactionType", 
                "description", 
                created_at 
            FROM transaction 
            WHERE 
                wallet_id = $1 
            ORDER BY created_at DESC
            LIMIT 10;"#,
            id
        )
        .fetch_all(&self.0)
        .await?;

        Ok(Statement {
            balance,
            last_transactions: transactions
                .into_iter()
                .map(|t| Transaction {
                    value: t.amount as u32,
                    transaction_type: t.r#type,
                    description: t.description,
                    date: t.created_at.and_utc(),
                })
                .collect(),
        })
    }
}

impl PostgresDatabase {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self(pool)
    }
}
