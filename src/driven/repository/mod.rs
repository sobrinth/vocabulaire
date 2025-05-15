use thiserror::Error;

pub mod mongo_repository;

#[derive(Error, Debug, PartialEq)]
pub enum RepoCreateError {
    #[error("Unknown")]
    Unknown,
}

#[derive(Error, Debug, PartialEq)]
pub enum RepoReadError {
    #[error("Not found")]
    NotFound,
    #[error("Unknown")]
    Unknown,
}

#[derive(Error, Debug, PartialEq)]
pub enum RepoUpdateError {
    #[error("malformed id")]
    BadId,
    #[error("Not Found")]
    NotFound,
    #[error("Unknown")]
    Unknown,
}

#[derive(Error, Debug, PartialEq)]
pub enum RepoDeleteError {
    #[error("malformed id")]
    BadId,
    #[error("Not Found")]
    NotFound,
    #[error("Unknown")]
    Unknown,
}
