use thiserror::Error;
use crate::domain::ports::TranslationRepository;
use crate::domain::voci::{Lang, TranslationRecord, TranslationRecordError, Word};
use crate::driven::repository::{RepoReadError, RepoUpdateError};

#[derive(Debug, PartialEq, Error)]
pub enum UpdateError {
    #[error("Bad Input: {0}")]
    WordError(#[from] TranslationRecordError),
    #[error("Read Error: {0}")]
    ReadError(#[from] RepoReadError),
    #[error("Update Error:")]
    UpdateError(#[from] RepoUpdateError),
}

/// Updates a translation record for a given word and language.
///
/// # Arguments
/// * `word` - The word to update the translation for
/// * `lang` - The language of the word
///
/// # Returns
/// * `Result<TranslationRecord, UpdateError>` - The updated translation record if successful,
///   or an error if the input is invalid
///
/// # Errors
/// Returns `UpdateError::WordError` if:
/// * The word is empty or invalid
/// * The language specification is invalid
pub async fn update_translation(
    repository: &impl TranslationRepository,
    word: &str,
    lang: &Lang,
    extra_translations: &Vec<&str>,
    extra_translation_lang: &Lang,
) -> Result<TranslationRecord, UpdateError> {
    let word = Word::new(word.to_string(), lang.clone())?;

    let mut tr_to_be_updated = repository.read_by_word(&word).await?;

    tr_to_be_updated.update(
        extra_translations
            .into_iter()
            .map(|t| t.to_string())
            .collect(),
        extra_translation_lang.clone(),
    )?;

    let updated_tr = repository.update(&tr_to_be_updated).await?;

    Ok(updated_tr)
}

#[cfg(test)]
mod tests {
    use actix_web::web::Data;

    use super::*;
    use crate::tests::{test_utils::shared::*, voci_repo_double::repo_double::VociRepoDouble};

    #[actix_rt::test]
    async fn update_existing_record_with_no_extra_word() {
        let repo = VociRepoDouble::new(&get_testing_persistence_config()).unwrap();

        let updated_tr = update_translation(
            &repo,
            &WORD,
            &WORD_LANG,
            &[].to_vec(),
            &TRANSLATION_LANG,
        )
        .await;

        let updated_translation = updated_tr.unwrap();
        let (_, _, _, actual_translations, _) = updated_translation.flat();
        assert_on_translations(
            actual_translations,
            &TRANSLATIONS.map(|t| t.to_string()).to_vec(),
        );
    }

    #[actix_rt::test]
    async fn update_existing_record() {
        let repo = VociRepoDouble::new(&get_testing_persistence_config()).unwrap();
        let mut expected = TRANSLATIONS.map(|t| t.to_string()).to_vec();

        expected.append(&mut ADDITONAL_TRANSLATIONS.map(|t| t.to_string()).to_vec());

        let updated_tr = update_translation(
            &repo,
            &WORD,
            &WORD_LANG,
            &ADDITONAL_TRANSLATIONS.to_vec(),
            &TRANSLATION_LANG,
        )
        .await;

        let updated_translation = updated_tr.unwrap();
        let (_, _, _, actual_translations, _) = updated_translation.flat();
        assert_on_translations(actual_translations, &expected);
    }
}
