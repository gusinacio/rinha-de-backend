use bson::Document;
use futures_util::StreamExt;
use mongodb::{Client, Database, IndexModel};
use mongodb::bson::{Bson, doc};
use mongodb::options::{CreateCollectionOptions, FindOneAndUpdateOptions, IndexOptions, ReturnDocument, ValidationAction, ValidationLevel};
use serde::{Deserialize, Serialize};

use crate::database::{Balance, models, Statement, Transaction, TransactionRepository, TransactionType};
use crate::error::ServerError;

#[derive(Clone)]
pub struct MongoDatabase {
    client: Client,
    database: Database,
}

impl MongoDatabase {
    pub async fn new(client: Client) -> Self {
        let database = client.default_database().expect("no default database");
        // assure that total cannot be less than -limit
        let validator = doc! {
            "$expr": doc! {
                "$gte": [
                    "$balance.total",
                    doc! {"$multiply": [ "$balance.limite", -1]},
                ]
            }
        };
        let validation_opts = CreateCollectionOptions::builder().validator(validator).validation_action(Some(ValidationAction::Error)).validation_level(Some(ValidationLevel::Strict)).build();
        database.create_collection(BALANCE, validation_opts).await.expect("failed to create collection");
        let collection = database.collection::<Statement>(BALANCE);
        let opts = IndexOptions::builder().unique(true).build();
        let model = IndexModel::builder().keys(doc! {"id": 1}).options(Some(opts)).build();
        collection.create_index(model, None).await.expect("failed to create index");
        migrate(&client).await;
        Self { client, database }
    }
}

const BALANCE: &str = "balances";
const TRANSACTIONS: &str = "transactions";

impl TransactionRepository for MongoDatabase {
    async fn add_transaction(&self, id: u32, transaction: Transaction) -> Result<Balance, ServerError> {
        let collection = self.database.collection::<MongoBalance>(BALANCE);
        let tx_value = transaction.value();
        let opts = FindOneAndUpdateOptions::builder().return_document(ReturnDocument::After).build();
        let value = collection.find_one_and_update(
            doc! {"id": id},
            doc! { "$inc": { "balance.total": tx_value }},
            Some(opts),
        ).await?;
        let mut balance = value.ok_or(ServerError::UserNotFound(id))?;
        balance.balance.statement_date = Some(chrono::Utc::now());

        let collection = self.database.collection::<MongoTransaction>(TRANSACTIONS);
        collection.insert_one(
            MongoTransaction {
                _id: bson::oid::ObjectId::new(),
                wallet_id: id,
                transaction,
            },
            None,
        ).await?;
        Ok(balance.balance)
    }

    async fn get_statement(&self, id: &u32) -> Result<Statement, ServerError> {
        let collection = self.database.collection::<MongoBalance>(BALANCE);
        let balance_match = doc! {
            "$match": doc! {
                "id": id
            }
        };
        let transactions_lookup = doc! {
            "$lookup": {
                "from": TRANSACTIONS,
                "as": "transactions",
                "let": doc! {"wallet_id": "$id"},
                "pipeline": vec![
                    doc! {
                        "$match": doc! {
                            "$expr": doc! {
                                "$eq": ["$wallet_id", "$$wallet_id"]
                            }
                        }
                    },
                    doc! {
                        "$sort": doc! {
                            "transaction.realizada_em": -1
                        }
                    },
                    doc! {
                        "$limit": 10
                    },
                    doc! {
                        "$replaceRoot": doc! {
                            "newRoot": "$transaction"
                        }
                    }
                ]
            }
        };
        let pipeline = vec![balance_match, transactions_lookup];
        let mut result = collection.aggregate(pipeline, None).await?;
        let result = result.next().await.ok_or(ServerError::UserNotFound(*id))??;

        let mut stmt: MongoStatement = bson::from_document(result)?;
        stmt.balance.statement_date = Some(chrono::Utc::now());
        Ok(Statement {
            balance: stmt.balance,
            last_transactions: stmt.transactions,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct MongoStatement {
    _id: bson::oid::ObjectId,
    id: u32,
    balance: Balance,
    transactions: Vec<Transaction>,
}

#[derive(Serialize, Deserialize)]
struct MongoBalance {
    _id: bson::oid::ObjectId,
    id: u32,
    balance: Balance,
}

#[derive(Serialize, Deserialize)]
struct MongoTransaction {
    _id: bson::oid::ObjectId,
    wallet_id: u32,
    transaction: Transaction,
}


async fn migrate(client: &Client) {
    let database = client.default_database().expect("no default database");
    let migration = database.collection::<Document>("migration");
    if migration.find_one(doc! {"migration": "v1"}, None).await.expect("failed to find migration").is_some() {
        return;
    }

    let collection = database.collection::<Document>(BALANCE);
    collection.insert_many(vec![
        doc! {"id": 1, "balance": Balance::new(100000)},
        doc! {"id": 2, "balance": Balance::new(80000)},
        doc! {"id": 3, "balance": Balance::new(1000000)},
        doc! {"id": 4, "balance": Balance::new(10000000)},
        doc! {"id": 5, "balance": Balance::new(500000)},
    ], None).await.expect("failed to insert documents");

    migration.insert_one(doc! {"migration": "v1"}, None).await.expect("failed to insert migration");
}

impl From<models::Statement> for Bson {
    fn from(value: Statement) -> Self {
        Bson::Document(doc! {
            "saldo": value.balance,
            "ultimas_transacoes": value.last_transactions,
        })
    }
}

impl From<models::Balance> for Bson {
    fn from(value: Balance) -> Self {
        Bson::Document(doc! {
            "total": value.total,
            "limite": value.limit,
        })
    }
}

impl From<models::TransactionType> for Bson {
    fn from(value: TransactionType) -> Self {
        match value {
            TransactionType::Deposit => Bson::String("c".to_string()),
            TransactionType::Withdraw => Bson::String("d".to_string()),
        }
    }
}

impl From<models::Transaction> for Bson {
    fn from(value: Transaction) -> Self {
        Bson::Document(doc! {
            "valor": value.value,
            "tipo": value.transaction_type,
            "descricao": value.description,
            "realizada_em": value.date,
        })
    }
}