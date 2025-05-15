use thiserror::Error;
use crate::domain::ports::TranslationRepository;
use crate::domain::voci::{Lang, TranslationRecord, TranslationRecordError, Word};
use crate::driven::repository::RepoReadError;

#[derive(Debug, PartialEq, Error)]
pub enum ReadError {
    #[error("Bad Input: {0}")]
    QueryWord(#[from] TranslationRecordError),
    #[error("Translation not found")]
    RecordNotFound,
    #[error("Unknown")]
    Unknown,
}

pub async fn read_translation(
    repository: &impl TranslationRepository,
    word: &str,
    lang: &Lang,
) -> Result<TranslationRecord, ReadError> {
    let word = Word::new(word.to_string(), lang.clone())?;

    let result = repository.read_by_word(&word).await;

    result.map_err(|e| match e {
        RepoReadError::NotFound => ReadError::RecordNotFound,
        RepoReadError::Unknown => ReadError::Unknown,
    })
}

#[cfg(test)]
mod tests {
    use actix_web::web::Data;

    use super::*;
    use crate::tests::{test_utils::shared::*, voci_repo_double::repo_double::VociRepoDouble};

    #[actix_rt::test]
    async fn read_well_formatted_word() {
        let repo = VociRepoDouble::new(&get_testing_persistence_config()).unwrap();

        let read_trans = read_translation(&repo, WORD, &WORD_LANG).await;

        assert_eq!(stub_translation_record(false), read_trans.unwrap())
    }

    #[actix_rt::test]
    async fn read_badly_formatted_word_err() {
        let repo = VociRepoDouble::new(&get_testing_persistence_config()).unwrap();

        let read_trans = read_translation(&repo, "", &WORD_LANG).await;

        assert_eq!(read_trans.is_err(), true);
        assert_eq!(
            read_trans.unwrap_err(),
            ReadError::QueryWord(TranslationRecordError::EmptyWord)
        );
    }
}
