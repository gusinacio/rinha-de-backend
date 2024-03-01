use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Statement {
    #[serde(rename = "saldo")]
    pub balance: Balance,
    #[serde(rename = "ultimas_transacoes")]
    pub last_transactions: Vec<Transaction>,
}

impl Balance {
    pub fn new(limit: u32) -> Self {
        Self {
            total: 0,
            statement_date: None,
            limit,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Balance {
    pub total: i32,
    #[serde(rename = "data_extrato")]
    pub statement_date: Option<chrono::DateTime<Utc>>,
    #[serde(rename = "limite")]
    pub limit: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    #[serde(rename = "valor")]
    pub value: u32,
    #[serde(rename = "tipo")]
    pub transaction_type: TransactionType,
    #[serde(rename = "descricao")]
    pub description: String,
    #[serde(rename = "realizada_em")]
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub date: chrono::DateTime<Utc>,
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

    pub fn value(&self) -> i32 {
        match self.transaction_type {
            TransactionType::Deposit => self.value as i32,
            TransactionType::Withdraw => -(self.value as i32),
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
