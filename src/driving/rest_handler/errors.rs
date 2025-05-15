use actix_web::{HttpResponse, error::ResponseError, http::StatusCode};
use thiserror::Error;

#[derive(Debug, PartialEq, Error)]
pub enum ApiError {
    #[error("Request cannot be handled")]
    BadRequest(String),
    #[error("Item not found")]
    NotFound(String),
    #[error("Input Invalid")]
    InvalidInput(String),
    #[error("Conflicting Item")]
    Conflict(String),
    #[error("Validation Error")]
    ValidationError(Vec<String>),
    #[error("Unknown")]
    Unknown(String),
}

/// Automatically convert ApiErrors to external ResponseError
impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::BadRequest(s) => HttpResponse::BadRequest().json(s),
            ApiError::NotFound(s) => HttpResponse::NotFound().json(s),
            ApiError::InvalidInput(s) => HttpResponse::BadRequest().json(s),
            ApiError::Conflict(s) => HttpResponse::Conflict().json(s),
            ApiError::ValidationError(s) => HttpResponse::UnprocessableEntity().json(s.to_vec()),
            ApiError::Unknown(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
