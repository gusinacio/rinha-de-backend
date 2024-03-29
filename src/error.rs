use axum::{extract::rejection::JsonRejection, http::StatusCode, response::IntoResponse};

#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("Invalid transaction")]
    FailedToSerialize(#[from] serde_json::Error),

    #[error("User not found {0}")]
    UserNotFound(u32),

    #[error("Transaction would exceed limit")]
    TransactionWouldExceedLimit,
    #[error("Failed to validate: {0}")]
    ValidationError(#[from] validator::ValidationErrors),
    #[error(transparent)]
    AxumFormRejection(#[from] JsonRejection),

    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),

    #[error("Redis error {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Mongo error {0}")]
    MongoError(#[from] mongodb::error::Error),

    #[error("Bson error {0}")]
    BsonError(#[from] bson::de::Error),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self {
            ServerError::UserNotFound(_) => StatusCode::NOT_FOUND,
            ServerError::RedisError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::ValidationError(_)
            | ServerError::FailedToSerialize(_)
            | ServerError::AxumFormRejection(_)
            | ServerError::SqlxError(_)
            | ServerError::MongoError(_)
            | ServerError::BsonError(_)
            | ServerError::TransactionWouldExceedLimit => StatusCode::UNPROCESSABLE_ENTITY,
        };
        (status_code, self.to_string()).into_response()
    }
}
