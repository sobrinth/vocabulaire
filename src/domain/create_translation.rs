use crate::domain::ports::TranslationRepository;
use crate::domain::voci::{Lang, TranslationRecord, TranslationRecordError};
use crate::driven::repository::{RepoCreateError, RepoReadError};
use thiserror::Error;

#[derive(Debug, PartialEq, Error)]
pub enum CreateError {
    #[error("Bad Input: {0}")]
    InvalidInput(#[from] TranslationRecordError),
    #[error("Read Error: {0}")]
    ReadError(#[from] RepoReadError),
    #[error("Create Error")]
    CreateError(#[from] RepoCreateError),
    #[error("Duplicate")]
    Duplicate,
}

pub async fn create_translation(
    repository: &impl TranslationRepository,
    word: &str,
    word_lang: &Lang,
    translations: &Vec<&str>,
    translation_lang: &Lang,
) -> Result<TranslationRecord, CreateError> {
    let tr = TranslationRecord::new(
        None,
        word.to_string(),
        word_lang.clone(),
        translations
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>(),
        translation_lang.clone(),
    )?;

    let word = tr.word();

    let does_exist: bool = repository.read_by_word(word).await.is_ok();

    if !does_exist {
        let create_response = repository.create(&tr).await?;
        Ok(create_response)
    } else {
        Err(CreateError::Duplicate)
    }
}
