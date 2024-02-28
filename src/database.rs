use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::ServerError;

#[derive(Clone)]
pub struct Database(Arc<RwLock<HashMap<u32, Statement>>>);

impl Database {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    async fn insert(&mut self, id: u32, statement: Statement) {
        self.0.write().await.insert(id, statement);
    }

    pub async fn add_transaction(
        &self,
        id: u32,
        transaction: Transaction,
    ) -> Result<Balance, ServerError> {
        let mut database = self.0.write().await;
        let stmt = database.get_mut(&id).ok_or(ServerError::UserNotFound(id))?;
        stmt.add_transaction(transaction)?;
        Ok(stmt.get_balance())
    }

    pub async fn get_statement(&self, id: &u32) -> Result<Statement, ServerError> {
        let mut stmt = self
            .0
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or(ServerError::UserNotFound(*id))?;
        stmt.last_transactions = stmt.only_last_10_transactions();
        Ok(stmt)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Statement {
    #[serde(rename = "saldo")]
    balance: Balance,
    #[serde(rename = "ultimas_transacoes")]
    last_transactions: Vec<Transaction>,
}

impl Statement {
    fn new(limit: u32) -> Self {
        Self {
            balance: Balance {
                total: 0,
                limit,
                statement_date: Utc::now(),
            },
            last_transactions: vec![],
        }
    }

    pub fn get_balance(&self) -> Balance {
        self.balance.clone()
    }

    pub fn update_statement_date(&mut self) {
        self.balance.statement_date = Utc::now();
    }

    pub fn only_last_10_transactions(&self) -> Vec<Transaction> {
        self.last_transactions
            .iter()
            .rev()
            .take(10)
            .cloned()
            .collect()
    }

    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<(), ServerError> {
        let value = self.balance.total
            + match transaction.transaction_type {
                TransactionType::Deposit => transaction.value as i32,
                TransactionType::Withdraw => -(transaction.value as i32),
            };
        if -value > self.balance.limit as i32 {
            Err(ServerError::TransactionWouldExceedLimit)
        } else {
            self.balance.total = value;
            self.last_transactions.push(transaction);
            Ok(())
        }
    }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TransactionType {
    #[serde(rename = "c")]
    Deposit,
    #[serde(rename = "d")]
    Withdraw,
}

pub async fn migrate(mut database: Database) {
    database.insert(1, Statement::new(100000)).await;
    database.insert(2, Statement::new(80000)).await;
    database.insert(3, Statement::new(1000000)).await;
    database.insert(4, Statement::new(10000000)).await;
    database.insert(5, Statement::new(500000)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::NewTransaction;

    #[test]
    fn should_not_exceed_limit() {
        let mut stmt = Statement::new(1000);
        let transaction =
            NewTransaction::new(1001, TransactionType::Withdraw, "description".to_string());
        let result = stmt.add_transaction(transaction.into());
        assert!(result.is_err());

        let transaction =
            NewTransaction::new(1000, TransactionType::Withdraw, "description".to_string());
        let result = stmt.add_transaction(transaction.into());
        assert!(result.is_ok());
    }
}
