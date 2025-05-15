use std::str::FromStr;

use async_trait::async_trait;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::doc;

use mongodb::{Client, Collection, bson};
use serde::{Deserialize, Serialize};

use crate::config::PersistenceConfig;
use crate::domain::ports::TranslationRepository;
use crate::domain::voci::{Lang, TranslationId, TranslationRecord, TranslationRecordError, Word};
use crate::driven::repository::RepoUpdateError;
use crate::driven::repository::RepoReadError;
use crate::driven::repository::RepoDeleteError;
use crate::driven::repository::RepoCreateError;

// Implement the `From<Lang> for Bson` trait
impl From<Lang> for bson::Bson {
    fn from(lang: Lang) -> Self {
        bson::Bson::String(match lang {
            Lang::fr => "fr".to_string(),
            Lang::de => "de".to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VociMongo {
    _id: ObjectId,
    word: String,
    lang: Lang,
    translations: Vec<String>,
    translation_lang: Lang,
}

impl From<TranslationRecord> for VociMongo {
    fn from(tr: TranslationRecord) -> Self {
        let object_id = match tr.id().value() {
            Some(id) => ObjectId::parse_str(id).unwrap(),
            None => ObjectId::new(),
        };

        let (_, word, lang, translations, translation_lang) = tr.flat();

        VociMongo {
            _id: object_id,
            word: word.clone(),
            lang: lang.clone(),
            translations: translations.clone(),
            translation_lang: translation_lang.clone(),
        }
    }
}

impl TryInto<TranslationRecord> for VociMongo {
    type Error = TranslationRecordError;
    fn try_into(self) -> Result<TranslationRecord, Self::Error> {
        TranslationRecord::new(
            Some(self._id.to_string()),
            self.word,
            self.lang,
            self.translations,
            self.translation_lang,
        )
    }
}

#[derive(Clone)]
pub struct VociMongoRepository {
    database: String,
    collection: String,
    conn_uri: String,
}

impl VociMongoRepository {
    async fn open_connection(&self) -> Client {
        let c = Client::with_uri_str(&self.conn_uri);
        c.await
            .expect("Error while opening the connection to MongoDB")
    }

    pub async fn get_collection(&self) -> Collection<VociMongo> {
        let client = self.open_connection().await;
        client.database(&self.database).collection(&self.collection)
    }

}

#[async_trait]
impl TranslationRepository for VociMongoRepository {
    fn new(config: &PersistenceConfig) -> Result<Self, String>
    where
        Self: Sized,
    {
        config.validate()?;
        let config = config.clone();

        let conn_uri = create_connection_uri(&config);

        Ok(VociMongoRepository {
            database: config.database,
            collection: config.schema_collection,
            conn_uri,
        })
    }

    async fn create(&self, tr: &TranslationRecord) -> Result<TranslationRecord, RepoCreateError> {
        let voci_mongo = VociMongo::from(tr.clone());
        let translation_collection = self.get_collection().await;

        let result = translation_collection.insert_one(voci_mongo).await;

        let inserted_id = match result {
            Ok(id) => id.inserted_id.as_object_id().unwrap(),
            Err(_e) => return Err(RepoCreateError::Unknown),
        };

        let (_, word, lang, translations, translation_lang) = tr.flat();

        let created_tr = TranslationRecord::new(
            Some(inserted_id.to_string()),
            word.clone(),
            lang.clone(),
            translations.clone(),
            translation_lang.clone(),
        )
        .unwrap();
        Ok(created_tr)
    }

    async fn read_by_word(&self, word: &Word) -> Result<TranslationRecord, RepoReadError> {
        let word = word.value();
        let doc = doc! {"word": word.0, "lang": word.1};

        let translation_collection = self.get_collection().await;

        let result = translation_collection.find_one(doc).await;

        let found = match result {
            Ok(v) => match v {
                Some(v) => v,
                None => return Err(RepoReadError::NotFound),
            },
            Err(_) => {
                return Err(RepoReadError::Unknown);
            }
        };

        found
            .try_into()
            .map_err(|e: TranslationRecordError| match e {
                _ => RepoReadError::Unknown,
            })
    }

    async fn update(&self, tr: &TranslationRecord) -> Result<TranslationRecord, RepoUpdateError> {
        let oid = match tr.id().value() {
            Some(v) => v,
            None => return Err(RepoUpdateError::BadId),
        };

        let object_id = match ObjectId::from_str(oid) {
            Ok(id) => id,
            Err(_) => return Err(RepoUpdateError::BadId),
        };

        let collection = self.get_collection().await;

        let res = collection
            .update_one(
                doc! {
                    "_id": object_id
                },
                doc! {
                    "$set": {
                        "translations": tr.flat().3              }
                },
            )
            .await;

        match res {
            Ok(r) => {
                if r.matched_count > 0 {
                    Ok(tr.clone())
                } else {
                    Err(RepoUpdateError::NotFound)
                }
            }
            Err(_) => Err(RepoUpdateError::Unknown),
        }
    }

    async fn delete(&self, id: &TranslationId) -> Result<(), RepoDeleteError> {
        let oid = match id.value() {

            Some(v) => v,
            None => return Err(RepoDeleteError::BadId),
        };
        let object_id = match ObjectId::from_str(oid) {
            Ok(id) => id,
            Err(_) => return Err(RepoDeleteError::BadId),
        };

        let collection = self.get_collection().await;

        let res = collection
            .delete_one(
                doc! {
                    "_id": object_id
                }
            )
            .await;

            match res {
                Ok(r) => {
                    if r.deleted_count > 0 {
                        Ok(())
                    } else {
                        Err(RepoDeleteError::NotFound)
                    }
                }
                Err(_) => Err(RepoDeleteError::Unknown),
            }
    }
}

fn create_connection_uri(config: &PersistenceConfig) -> String {
    format!(
        "mongodb://{}:{}@{}/{}",
        config.user,
        config.password,
        match config.port {
            None => config.host.to_string(),
            Some(port) => config.host.clone() + ":" + &port.to_string(),
        },
        config.auth_db
    )
}

#[cfg(test)]
mod tests {
    use crate::tests::test_utils::shared::{
        ADDITONAL_TRANSLATIONS, assert_on_translation_record, get_testing_persistence_config,
        stub_translation_record, setup_repo,
    };
    use serial_test::serial;

    use super::*;

    #[serial]
    #[actix_rt::test]
    async fn new_repo_ok_config_repo_created() {
        let repo = setup_repo().await;
        assert_eq!(
            repo.conn_uri,
            "mongodb://root:tran5lation5@localhost:27017/admin"
        );
        assert_eq!(repo.collection, "test_translation")
    }

    #[test]
    #[serial]
    fn new_repo_bad_config_error() {
        let mut config = get_testing_persistence_config();
        config.host = "".to_string();
        let result: Result<VociMongoRepository, String> =
            VociMongoRepository::new(&config);

        assert_eq!(result.is_err(), true);
    }

    #[serial]
    #[actix_rt::test]
    async fn create_ok_parameters_record_created() {
        let repo = setup_repo().await;

        let result = repo.create(&stub_translation_record(false)).await.unwrap();
        let expected = stub_translation_record(true);

        assert_on_translation_record(&result, &expected, false);
    }

    #[serial]
    #[actix_rt::test]
    async fn read_by_existing_word_return_translation_record() {
        let repo = setup_repo().await;
        let tr = stub_translation_record(false);
        repo.create(&tr).await.unwrap();

        let inserted_word = tr.word();
        let result = repo.read_by_word(inserted_word).await.unwrap();

        assert_on_translation_record(&result, &tr, false);
    }

    #[serial]
    #[actix_rt::test]
    async fn read_by_nonexisting_word_return_notfounderror() {
        let repo = setup_repo().await;
        let tr = stub_translation_record(false);
        let _ = repo.create(&tr).await.unwrap();

        let non_existing_word = Word::new("nix".to_string(), Lang::de).unwrap();
        let result = repo.read_by_word(&non_existing_word).await;

        assert_eq!(result.is_err(), true);
        assert_eq!(result.unwrap_err(), RepoReadError::NotFound);
    }

    #[serial]
    #[actix_rt::test]
    async fn update_ok_record_return_updated_record() {
        let repo = setup_repo().await;
        let mut tr = repo.create(&stub_translation_record(false)).await.unwrap();
        let extra_translations = ADDITONAL_TRANSLATIONS.map(|r| r.to_string()).to_vec();
        let mut expected_tr = tr.clone();
        let _ = expected_tr.update(extra_translations.clone(), Lang::de);
        let _ = tr.update(extra_translations, Lang::de);

        let updated_tr = repo.update(&tr).await.unwrap();

        assert_eq!(updated_tr, expected_tr);
    }

    #[serial]
    #[actix_rt::test]
    async fn update_without_id_record_return_error() {
        let repo = setup_repo().await;
        let tr = &stub_translation_record(false);
        let _ = repo.create(&tr).await.unwrap();

        let updated_tr = repo.update(&tr).await;

        assert_eq!(updated_tr.unwrap_err(), RepoUpdateError::BadId);
    }

    #[serial]
    #[actix_rt::test]
    async fn delete_existing_id_ok() {
        let repo = setup_repo().await;
        let tr = &stub_translation_record(false);
        let created_tr = repo.create(&tr).await.unwrap();

        let delete_id = created_tr.id();

        let del_res = repo.delete(&delete_id).await;

        assert_eq!(del_res, Ok(()));
    }

    #[serial]
    #[actix_rt::test]
    async fn delete_non_existing_id_error() {
        let repo = setup_repo().await;
        let tr = &stub_translation_record(false);
        let _ = repo.create(&tr).await.unwrap();

        let delete_id = TranslationId::from("6817c21bf99716ff3f9968eb".to_string());

        assert_eq!(
            repo.delete(&delete_id).await.unwrap_err(),
            RepoDeleteError::NotFound
        );
    }

    #[serial]
    #[actix_rt::test]
    async fn delete_none_id_error() {
        let repo = setup_repo().await;
        let tr = &stub_translation_record(false);
        let _ = repo.create(&tr).await.unwrap();

        let delete_id = TranslationId::from("".to_string());

        assert_eq!(
            repo.delete(&delete_id).await.unwrap_err(),
            RepoDeleteError::BadId
        );
    }

    // async fn setup_repo() -> VociMongoRepository {
    //     let config = get_testing_persistence_config();
    //     let repo: VociMongoRepository = Repository::<TranslationRecord>::new(&config).unwrap();

    //     delete_collection(config, &repo).await;

    //     repo
    // }

    // async fn delete_collection(config: PersistenceConfig, repo: &VociMongoRepository) {
    //     let collection = repo.get_collection().await;
    //     let coll: Collection<VociMongoRepository> = collection
    //         .client()
    //         .database(&config.database)
    //         .collection(&config.schema_collection);
    //     coll.delete_many(doc! {}).await.unwrap();
    // }
}