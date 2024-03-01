use bson::Document;
use mongodb::{Client, Database, IndexModel};
use mongodb::bson::{Bson, doc};
use mongodb::options::{CreateCollectionOptions, FindOneAndUpdateOptions, FindOneOptions, IndexOptions, ReturnDocument, ValidationAction, ValidationLevel};
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
                    "$statement.saldo.total",
                    doc! {"$multiply": [ "$statement.saldo.limite", -1]},
                ]
            }
        };
        let validation_opts = CreateCollectionOptions::builder().validator(validator).validation_action(Some(ValidationAction::Error)).validation_level(Some(ValidationLevel::Strict)).build();
        database.create_collection(COLLECTION, validation_opts).await.expect("failed to create collection");
        let collection = database.collection::<Statement>(COLLECTION);
        let opts = IndexOptions::builder().unique(true).build();
        let model = IndexModel::builder().keys(doc! {"id": 1}).options(Some(opts)).build();
        collection.create_index(model, None).await.expect("failed to create index");
        migrate(&client).await;
        Self { client, database }
    }
}

const COLLECTION: &str = "statements";

impl TransactionRepository for MongoDatabase {
    async fn add_transaction(&self, id: u32, transaction: Transaction) -> Result<Balance, ServerError> {
        let collection = self.database.collection::<Balance>(COLLECTION);
        let tx_value = transaction.value();
        let projection = doc! {
            "statement.saldo": true
        };
        let opts = FindOneAndUpdateOptions::builder().projection(projection).return_document(ReturnDocument::After).build();
        let value = collection.find_one_and_update(
            doc! {"id": id},
            doc! {"$push": {"statement.ultimas_transacoes": doc! {
                "$each": [transaction],
                    "$position": 0,
                }
            }, "$inc": {"statement.saldo.total": tx_value }},
            Some(opts),
        ).await?;
        let mut balance = value.ok_or(ServerError::UserNotFound(id))?;
        balance.statement_date = Some(chrono::Utc::now());
        Ok(balance)
    }

    async fn get_statement(&self, id: &u32) -> Result<Statement, ServerError> {
        let collection = self.database.collection::<MongoStatement>(COLLECTION);
        let projection = doc! {
            "statement.ultimas_transacoes": {"$slice": 10}
        };
        let opts = FindOneOptions::builder().projection(Some(projection)).build();
        let stmt = collection.find_one(doc! {"id": id}, Some(opts)).await?;
        let mut stmt = stmt.ok_or(ServerError::UserNotFound(*id)).map(|stmt| stmt.statement)?;
        stmt.balance.statement_date = Some(chrono::Utc::now());
        Ok(stmt)
    }
}

#[derive(Serialize, Deserialize)]
struct MongoStatement {
    _id: bson::oid::ObjectId,
    id: u32,
    statement: Statement,
}


async fn migrate(client: &Client) {
    let database = client.default_database().expect("no default database");
    let collection = database.collection::<Document>(COLLECTION);

    // INSERT INTO wallet ("limit", total) VALUES (100000, 0);
    // INSERT INTO wallet ("limit", total) VALUES (80000, 0);
    // INSERT INTO wallet ("limit", total) VALUES (1000000, 0);
    // INSERT INTO wallet ("limit", total) VALUES (10000000, 0);
    // INSERT INTO wallet ("limit", total) VALUES (500000, 0);

    collection.insert_many(vec![
        doc! {"id": 1, "statement": Statement::new(100000)},
        doc! {"id": 2, "statement": Statement::new(80000)},
        doc! {"id": 3, "statement": Statement::new(1000000)},
        doc! {"id": 4, "statement": Statement::new(10000000)},
        doc! {"id": 5, "statement": Statement::new(500000)},
    ], None).await.expect("failed to insert documents");
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