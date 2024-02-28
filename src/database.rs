use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

use crate::error::ServerError;

#[derive(Clone)]
pub struct Database(Pool<Postgres>);

impl Database {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self(pool)
    }

    pub async fn add_transaction(
        &self,
        id: u32,
        transaction: Transaction,
    ) -> Result<Balance, ServerError> {
        let mut tx = self.0.begin().await?;
        let balance = sqlx::query!(
            "UPDATE wallet SET total = total + $2 WHERE id = $1 RETURNING *;",
            id as i32,
            match transaction.transaction_type {
                TransactionType::Deposit => transaction.value as i32,
                TransactionType::Withdraw => -(transaction.value as i32),
            }
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
            statement_date: Utc::now(),
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

    pub async fn get_statement(&self, id: &u32) -> Result<Statement, ServerError> {
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
            statement_date: Utc::now(),
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Statement {
    #[serde(rename = "saldo")]
    balance: Balance,
    #[serde(rename = "ultimas_transacoes")]
    last_transactions: Vec<Transaction>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Balance {
    pub total: i32,
    #[serde(rename = "data_extrato")]
    statement_date: chrono::DateTime<Utc>,
    #[serde(rename = "limite")]
    pub limit: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    #[serde(rename = "valor")]
    value: u32,
    #[serde(rename = "tipo")]
    transaction_type: TransactionType,
    #[serde(rename = "descricao")]
    description: String,
    #[serde(rename = "realizada_em")]
    date: chrono::DateTime<Utc>,
}

impl Transaction {
    pub fn new(value: u32, transaction_type: TransactionType, description: String) -> Self {
        Self {
            value,
            transaction_type,
            description,
            date: Utc::now(),
        }
    }
}

#[derive(sqlx::Type, Debug, Serialize, Deserialize, Clone)]
#[sqlx(type_name = "transaction_type")]
pub enum TransactionType {
    #[serde(rename = "c")]
    #[sqlx(rename = "c")]
    Deposit,
    #[serde(rename = "d")]
    #[sqlx(rename = "d")]
    Withdraw,
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
