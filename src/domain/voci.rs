use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// Represents available languages in the system
/// Languages codes according to https://de.wikipedia.org/wiki/Liste_der_ISO-639-2-Codes
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Lang {
    fr, // french
    de, // german
}

#[derive(Debug, PartialEq, Error)]
pub enum TranslationRecordError {
    #[error("Word is Empty")]
    EmptyWord,
    #[error("No Translation")]
    EmptyTranslation,
    #[error("One of the Words in Translations is empty")]
    EmptyWordInTranslation,
    #[error("Translation language mismatch")]
    TranslationLanguageMismatch,
    #[error("Update with same items")]
    UpdateWithSameItems,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationId(Option<String>);

impl TranslationId {
    pub fn value(&self) -> &Option<String> {
        &self.0
    }
}

impl From<String> for TranslationId {
    fn from(id: String) -> Self {
        if id.is_empty() {
            Self(None)
        } else {
            Self(Some(id))
        }
    }
}
impl From<Option<String>> for TranslationId {
    fn from(opt: Option<String>) -> Self {
        match opt {
            Some(val) => {
                if val.is_empty() {
                    Self(None)
                } else {
                    Self(Some(val.to_string()))
                }
            }
            None => Self(None),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    word: String,
    lang: Lang,
}

impl Word {
    pub fn new(word: String, lang: Lang) -> Result<Self, TranslationRecordError> {
        if word.is_empty() {
            return Err(TranslationRecordError::EmptyWord);
        }
        Ok(Word { word, lang })
    }
    pub fn value(&self) -> (&String, &Lang) {
        (&self.word, &self.lang)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Translations {
    lang: Lang,
    words: Vec<String>,
}

impl Translations {
    fn new(words: Vec<String>, lang: Lang) -> Result<Self, TranslationRecordError> {
        if words.is_empty() {
            return Err(TranslationRecordError::EmptyTranslation);
        }

        if words.iter().any(|s| s.is_empty()) {
            return Err(TranslationRecordError::EmptyWordInTranslation);
        }

        Ok(Translations { words, lang })
    }

    fn translations(&self) -> &Vec<String> {
        &self.words
    }
    fn value(&self) -> (&Vec<String>, &Lang) {
        (self.translations(), &self.lang)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationRecord {
    id: TranslationId,
    word: Word,
    translations: Translations,
}

impl TranslationRecord {
    pub fn new(
        id: Option<String>,
        word: String,
        word_lang: Lang,
        translations: Vec<String>,
        translation_lang: Lang,
    ) -> Result<Self, TranslationRecordError> {
        let id = TranslationId::from(id);
        let word = Word::new(word, word_lang)?;
        let translations = Translations::new(translations, translation_lang)?;

        Ok(TranslationRecord {
            id,
            word,
            translations,
        })
    }

    pub fn id(&self) -> &TranslationId {
        &self.id
    }

    pub fn word(&self) -> &Word {
        &self.word
    }

    pub fn update(
        &mut self,
        translations: Vec<String>,
        lang: Lang,
    ) -> Result<(), TranslationRecordError> {
        if self.translations.lang != lang {
            return Err(TranslationRecordError::TranslationLanguageMismatch);
        }

        if vectors_are_equal(&self.translations.words, &translations) {
            return Err(TranslationRecordError::UpdateWithSameItems);
        }

        let mut seen: HashSet<String> = self.translations.words.iter().cloned().collect();

        for t in translations {
            if !t.is_empty() && !seen.contains(&t) {
                seen.insert(t.clone());
                self.translations.words.push(t);
            }
        }

        Ok(())
    }

    pub fn flat(&self) -> (&Option<String>, &String, &Lang, &Vec<String>, &Lang) {
        let id = &self.id.value();
        let word = &self.word.value();
        let trans = &self.translations.value();
        (id, word.0, word.1, trans.0, trans.1)
    }
}

fn vectors_are_equal<T: Ord + Clone>(vec1: &[T], vec2: &[T]) -> bool {
    if vec1.len() != vec2.len() {
        return false;
    }

    let mut sorted_vec1 = vec1.to_vec();
    let mut sorted_vec2 = vec2.to_vec();

    sorted_vec1.sort();
    sorted_vec2.sort();

    sorted_vec1 == sorted_vec2
}

#[cfg(test)]
mod tests {
    use crate::tests::test_utils::shared::{
        ADDITONAL_TRANSLATIONS, TRANSLATIONS, stub_translation_record,
    };

    use super::*;

    #[test]
    fn word_new_ok_input_constructed() {
        let word = Word::new("chien".to_string(), Lang::fr);
        assert_eq!((word.unwrap().value()), (&"chien".to_string(), &Lang::fr));
    }

    #[test]
    fn word_new_bad_input_error() {
        let err_word = Word::new("".to_string(), Lang::fr);
        assert_eq!(err_word.is_err(), true);
        assert_eq!(err_word.unwrap_err(), TranslationRecordError::EmptyWord);
    }

    #[test]
    fn translation_new_ok_input_constructed() {
        let words = vec!["hund".to_string(), "köter".to_string()];

        let translations = Translations::new(words.clone(), Lang::de);
        for (i, translation) in translations.unwrap().translations().into_iter().enumerate() {
            assert_eq!(*translation, words[i]);
        }
    }

    #[test]
    fn translation_new_empty_word_err() {
        let err_words = vec!["".to_string(), "köter".to_string()];

        let err_translations = Translations::new(err_words.clone(), Lang::de);
        assert_eq!(err_translations.is_err(), true);
        assert_eq!(
            err_translations.unwrap_err(),
            TranslationRecordError::EmptyWordInTranslation
        );
    }

    #[test]
    fn translation_new_empty_string_err() {
        let err_words = vec![];

        let err_translations = Translations::new(err_words.clone(), Lang::de);
        assert_eq!(err_translations.is_err(), true);
        assert_eq!(
            err_translations.unwrap_err(),
            TranslationRecordError::EmptyTranslation
        );
    }

    #[test]
    fn translation_record_new_ok_input_constructed() {
        let id = Some("1234".to_string());

        let word = "chien".to_string();
        let word_lang = Lang::fr;

        let translations = vec!["hund".to_string(), "köter".to_string()];
        let translation_lang = Lang::de;

        let chien = TranslationRecord::new(
            id.clone(),
            word,
            word_lang,
            translations.clone(),
            translation_lang,
        )
        .unwrap();

        assert_eq!(*chien.id.value(), id);
        assert_eq!(chien.word.word, "chien");
        assert_eq!(chien.word.lang, Lang::fr);
        assert_eq!(chien.translations.lang, Lang::de);
        for (i, translation) in chien.translations.words.into_iter().enumerate() {
            assert_eq!(translation, translations[i]);
        }
    }

    #[test]
    fn translation_record_new_bad_input_err() {
        let word = "chien".to_string();
        let word_lang = Lang::fr;

        let translations = vec!["hund".to_string(), "".to_string()];
        let translation_lang = Lang::de;

        let chien = TranslationRecord::new(
            None,
            word,
            word_lang,
            translations.clone(),
            translation_lang,
        );

        assert_eq!(chien.is_err(), true);
        assert_eq!(
            chien.unwrap_err(),
            TranslationRecordError::EmptyWordInTranslation
        );
    }

    #[test]
    fn translation_record_update_correct_translation_record() {
        let mut tr = stub_translation_record(true);
        let extra_translations = ADDITONAL_TRANSLATIONS.map(|r| r.to_string()).to_vec();
        let mut expected = tr.flat().3.clone();
        expected.append(&mut extra_translations.clone());

        let _ = tr.update(extra_translations, Lang::de);

        assert_eq!(tr.flat().3, &expected);
    }

    #[test]
    fn translation_record_update_different_language_no_update_and_error() {
        let mut tr = stub_translation_record(true);
        let extra_translations = ADDITONAL_TRANSLATIONS.map(|r| r.to_string()).to_vec();
        let expected = tr.flat().3.clone();

        let result = tr.update(extra_translations, Lang::fr);

        assert_eq!(tr.flat().3, &expected);
        assert_eq!(
            result.unwrap_err(),
            TranslationRecordError::TranslationLanguageMismatch
        );
    }

    #[test]
    fn translation_record_update_same_word_as_already_in_no_update() {
        let mut tr = stub_translation_record(true);
        let extra_translations = vec![TRANSLATIONS[0].to_string()];
        let expected = tr.flat().3.clone();

        let _ = tr.update(extra_translations, Lang::de);

        assert_eq!(tr.flat().3, &expected);
    }

    #[test]
    fn translation_record_update_same_word_twice_in_update_no_update() {
        let mut tr = stub_translation_record(true);
        let extra_translations = vec![TRANSLATIONS[0].to_string(), TRANSLATIONS[0].to_string()];
        let expected = tr.flat().3.clone();

        let _ = tr.update(extra_translations, Lang::de);

        assert_eq!(tr.flat().3, &expected);
    }

    #[test]
    fn translation_record_update_with_existing_words_no_update_error() {
        let mut tr = stub_translation_record(true);
        let extra_translations = vec![TRANSLATIONS[1].to_string(), TRANSLATIONS[0].to_string()];
        let expected = tr.flat().3.clone();

        let result = tr.update(extra_translations, Lang::de);

        assert_eq!(tr.flat().3, &expected);
        assert_eq!(
            result.unwrap_err(),
            TranslationRecordError::UpdateWithSameItems
        );
    }
}
