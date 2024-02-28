use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    database::{Balance, Database, Statement, Transaction, TransactionType},
    error::ServerError,
    validator::ValidatedJson,
};

#[derive(Debug, Serialize, Deserialize, Validate)]
pub(crate) struct NewTransaction {
    #[serde(rename = "valor")]
    value: u32,
    #[serde(rename = "tipo")]
    transaction_type: TransactionType,
    #[serde(rename = "descricao")]
    #[validate(length(min = 1, max = 10))]
    description: String,
}

impl From<NewTransaction> for Transaction {
    fn from(transaction: NewTransaction) -> Self {
        Self::new(
            transaction.value,
            transaction.transaction_type,
            transaction.description,
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NewTransactionResponse {
    #[serde(rename = "limite")]
    limit: u32,
    #[serde(rename = "saldo")]
    balance: i32,
}

impl From<Balance> for NewTransactionResponse {
    fn from(balance: Balance) -> Self {
        Self {
            limit: balance.limit,
            balance: balance.total,
        }
    }
}

async fn get_statement(
    Path(id): Path<u32>,
    database: State<Database>,
) -> Result<Json<Statement>, ServerError> {
    let stmt = database.get_statement(&id).await?;
    Ok(Json(stmt))
}

async fn post_transaction(
    Path(id): Path<u32>,
    State(database): State<Database>,
    ValidatedJson(transaction): ValidatedJson<NewTransaction>,
) -> Result<Json<NewTransactionResponse>, ServerError> {
    transaction.validate()?;
    let balance = database.add_transaction(id, transaction.into()).await?;
    Ok(Json(balance.into()))
}

pub fn client_router() -> Router<Database> {
    Router::new()
        .route("/:id/extrato", get(get_statement))
        .route("/:id/transacoes", post(post_transaction))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Result;

    impl NewTransaction {
        pub(crate) fn new(
            value: u32,
            transaction_type: TransactionType,
            description: String,
        ) -> Self {
            Self {
                value,
                transaction_type,
                description,
            }
        }
    }

    #[test]
    fn test_transaction() {
        let transaction = r#"
            {
                "valor": 1000,
                "tipo" : "c",
                "descricao" : "descricao"
            }
        "#;

        let transaction: Result<NewTransaction> = serde_json::from_str(transaction);
        assert!(transaction.is_ok());
    }
}
