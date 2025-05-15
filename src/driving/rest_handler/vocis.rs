use actix_web::web::Json;
use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::domain::create_translation::CreateError;
use crate::domain::delete_translation::DeleteError;
use crate::domain::read_translation::ReadError;
use crate::domain::update_translation::UpdateError;
use crate::domain::voci::{Lang, TranslationRecord};
use crate::{Repository, domain};
use crate::domain::ports::TranslationRepository;
use crate::driving::rest_handler::errors::ApiError;
use crate::driving::rest_handler::validate::validate;

/// Helper function to reduce boilerplate of an OK/Json response
fn respond_json<T>(data: T) -> Result<Json<T>, ApiError>
where
    T: Serialize,
{
    Ok(Json(data))
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct TranslationResponse {
    pub id: Option<String>,
    pub word: String,
    pub lang: Lang,
    pub translations: Vec<String>,
    pub translation_lang: Lang,
}
impl From<TranslationRecord> for TranslationResponse {
    fn from(s: TranslationRecord) -> Self {
        let (id, word, lang, translations, translation_lang) = s.flat();
        TranslationResponse {
            id: id.clone(),
            word: word.clone(),
            lang: lang.clone(),
            translations: translations.clone(),
            translation_lang: translation_lang.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct CreateTranslationRequest {
    pub id: Option<String>,
    #[validate(length(min = 1, message = "Word is required and must be at least 1 character"))]
    pub word: String,
    pub lang: Lang,
    #[validate(length(
        min = 1,
        message = "ingredients is required and must be at least 1 item"
    ))]
    pub translations: Vec<String>,
    pub translation_lang: Lang,
}

pub async fn create_translation<T: TranslationRepository>(
    repository: web::Data<T>,
    request: Json<CreateTranslationRequest>,
) -> Result<Json<TranslationResponse>, ApiError> {
    validate(&request)?;

    let result = domain::create_translation::create_translation(
        repository.get_ref(),
        &request.word,
        &request.lang,
        &request.translations.iter().map(|s| s.as_str()).collect(),
        &request.translation_lang,
    )
    .await;

    result
        .map(|v| respond_json(TranslationResponse::from(v)))
        .map_err(|e| match e {
            CreateError::InvalidInput(s) => ApiError::InvalidInput(s.to_string()),
            CreateError::ReadError(s) => ApiError::NotFound(s.to_string()),
            CreateError::CreateError(s) => ApiError::BadRequest(s.to_string()),
            CreateError::Duplicate => ApiError::Conflict(e.to_string()),
        })?
}

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct RequestTranslationByWord {
    #[validate(length(min = 1, message = "Word is required and must be at least 1 character"))]
    pub word: String,
    pub lang: Lang,
}

pub async fn read_translation<T: TranslationRepository>(
    repository: web::Data<T>,
    request: Json<RequestTranslationByWord>,
) -> Result<Json<TranslationResponse>, ApiError> {
    validate(&request)?;
    
    let result: Result<TranslationRecord, ReadError> =
        domain::read_translation::read_translation(repository.get_ref(), &request.word, &request.lang).await;

    result
        .map(|v| respond_json(TranslationResponse::from(v)))
        .map_err(|e| match e {
            ReadError::QueryWord(s) => ApiError::InvalidInput(s.to_string()),
            ReadError::RecordNotFound => ApiError::NotFound(e.to_string()),
            ReadError::Unknown => ApiError::Unknown(e.to_string()),
        })?
}

pub async fn update_translation<T: TranslationRepository>(
    repository: web::Data<T>,
    request: Json<CreateTranslationRequest>,
) -> Result<Json<TranslationResponse>, ApiError> {
    validate(&request)?;

    let result = domain::update_translation::update_translation(
        repository.get_ref(),
        &request.word,
        &request.lang,
        &request.translations.iter().map(|s| s.as_str()).collect(),
        &request.translation_lang,
    )
    .await;

    result
        .map(|v| respond_json(TranslationResponse::from(v)))
        .map_err(|e| match e {
            UpdateError::WordError(s) => ApiError::InvalidInput(s.to_string()),
            UpdateError::ReadError(s) => ApiError::NotFound(s.to_string()),
            UpdateError::UpdateError(s) => ApiError::NotFound(s.to_string()),
        })?
}

pub async fn delete_translation<T: TranslationRepository>(
    repository: web::Data<T>,
    request: Json<RequestTranslationByWord>,
) -> Result<HttpResponse, ApiError> {
    validate(&request)?;

    let result =
        domain::delete_translation::delete_translation(repository.get_ref(), &request.word, &request.lang)
            .await;

    result
        .map(|_| Ok(HttpResponse::Ok().finish()))
        .map_err(|e| match e {
            DeleteError::WordError(s) => ApiError::InvalidInput(s.to_string()),
            DeleteError::ReadError(s) => ApiError::InvalidInput(s.to_string()),
            DeleteError::DeleteError(s) => ApiError::Unknown(s.to_string()),
        })?
}

#[cfg(test)]
mod tests {
    use actix_web::test::TestRequest;
    use actix_web::{
        App, FromRequest, Handler, Responder, Route, http::StatusCode, test, web::Data,
    };
    use serial_test::serial;

    use super::*;
    use crate::driven::repository::mongo_repository::VociMongoRepository;
    use crate::tests::test_utils::shared::*;

    #[serial]
    #[actix_web::test]
    async fn create_translation_ok_input_same_translation_returned() {
        let repo = setup_repo().await;

        let create_req = CreateTranslationRequest {
            id: None,
            word: WORD.to_string(),
            lang: WORD_LANG,
            translations: stub_translations(),
            translation_lang: TRANSLATION_LANG,
        };

        let resp: TranslationResponse = execute(
            &repo,
            "/",
            None,
            web::post(),
            TestRequest::post(),
            create_translation::<VociMongoRepository>,
            Some(create_req),
        )
        .await;

        let expected = TranslationRecord::new(
            Some(TRANSLATION_ID.to_string()),
            WORD.to_string(),
            WORD_LANG,
            stub_translations(),
            TRANSLATION_LANG,
        )
        .unwrap();

        assert_on_translation_response(&resp, &expected, false);
    }

    #[should_panic]
    #[serial]
    #[actix_web::test]
    async fn create_translation_word_already_exists_panic() {
        let repo = setup_repo().await;
        let create_req = CreateTranslationRequest {
            id: None,
            word: WORD.to_string(),
            lang: WORD_LANG,
            translations: stub_translations(),
            translation_lang: TRANSLATION_LANG,
        };
        let _response: TranslationResponse = execute(
            &repo,
            "/",
            None,
            web::post(),
            TestRequest::post(),
            create_translation::<VociMongoRepository>,
            Some(create_req),
        )
        .await;

        let doublet_request = CreateTranslationRequest {
            id: None,
            word: WORD.to_string(),
            lang: WORD_LANG,
            translations: stub_translations(),
            translation_lang: Lang::fr,
        };
        let _: TranslationResponse = execute(
            &repo,
            "/",
            None,
            web::post(),
            TestRequest::post(),
            create_translation::<VociMongoRepository>,
            Some(doublet_request),
        )
        .await;
    }

    #[serial]
    #[actix_web::test]
    async fn create_translation_word_already_exists_http_conflict() {
        let repo = setup_repo().await;

        let create_req = CreateTranslationRequest {
            id: None,
            word: WORD.to_string(),
            lang: WORD_LANG,
            translations: stub_translations(),
            translation_lang: TRANSLATION_LANG,
        };
        let _response: TranslationResponse = execute(
            &repo,
            "/",
            None,
            web::post(),
            TestRequest::post(),
            create_translation::<VociMongoRepository>,
            Some(create_req),
        )
        .await;

        let doublet_request = CreateTranslationRequest {
            id: None,
            word: WORD.to_string(),
            lang: WORD_LANG,
            translations: stub_translations(),
            translation_lang: Lang::fr,
        };
        let create_rep = execute_http(
            &repo,
            "/",
            None,
            web::post(),
            TestRequest::post(),
            create_translation::<VociMongoRepository>,
            Some(doublet_request),
        )
        .await;

        assert_eq!(create_rep.status(), StatusCode::CONFLICT);
    }

    #[serial]
    #[actix_web::test]
    async fn read_translation_by_word_good_input_translation_returned() {
        let repo = setup_repo().await;
        let create_req = CreateTranslationRequest {
            id: None,
            word: WORD.to_string(),
            lang: WORD_LANG,
            translations: stub_translations(),
            translation_lang: TRANSLATION_LANG,
        };
        let _: TranslationResponse = execute(
            &repo,
            "/",
            None,
            web::post(),
            TestRequest::post(),
            create_translation::<VociMongoRepository>,
            Some(create_req),
        )
        .await;

        let read_req = RequestTranslationByWord {
            word: WORD.to_string(),
            lang: WORD_LANG,
        };

        let resp: TranslationResponse = execute(
            &repo,
            "/",
            None,
            web::get(),
            TestRequest::get(),
            read_translation::<VociMongoRepository>,
            Some(read_req),
        )
        .await;

        let expected = TranslationRecord::new(
            Some(TRANSLATION_ID.to_string()),
            WORD.to_string(),
            WORD_LANG,
            stub_translations(),
            TRANSLATION_LANG,
        )
        .unwrap();

        assert_on_translation_response(&resp, &expected, false);
    }

    #[serial]
    #[actix_web::test]
    async fn delete_translation_ok_input_http_success() {
        let repo = setup_repo().await;

        let create_req = CreateTranslationRequest {
            id: None,
            word: WORD.to_string(),
            lang: WORD_LANG,
            translations: stub_translations(),
            translation_lang: TRANSLATION_LANG,
        };
        let _: TranslationResponse = execute(
            &repo,
            "/",
            None,
            web::post(),
            TestRequest::post(),
            create_translation::<VociMongoRepository>,
            Some(create_req),
        )
        .await;

        let del_req = RequestTranslationByWord {
            word: WORD.to_string(),
            lang: WORD_LANG,
        };
        let r = execute_http(
            &repo,
            "/",
            None,
            web::delete(),
            TestRequest::delete(),
            delete_translation::<VociMongoRepository>,
            Some(del_req),
        )
        .await;

        assert_eq!(r.status().is_success(), true);
    }

    #[serial]
    #[actix_web::test]
    async fn delete_translation_bad_input_http_client_error() {
        let repo = setup_repo().await;
        let del_req = RequestTranslationByWord {
            word: "".to_string(),
            lang: WORD_LANG,
        };

        let r = execute_http(
            &repo,
            "/",
            None,
            web::delete(),
            TestRequest::delete(),
            delete_translation::<VociMongoRepository>,
            Some(del_req),
        )
        .await;

        assert_eq!(r.status().is_client_error(), true);
    }

    /// Execute a test request and return HttpResponse
    async fn execute_http<F, Args, R>(
        repo: &R,
        path: &str,
        uri_to_call: Option<&str>,
        http_method: Route,
        test_req: TestRequest,
        handler: F,
        recipe_req: Option<impl Serialize>,
    ) -> HttpResponse
    where
        R: TranslationRepository,
        F: Handler<Args>,
        Args: FromRequest + 'static,
        F::Output: Responder,
    {
        // init the service
        let app = test::init_service(
            App::new()
                .app_data(Data::new(repo.clone()))
                .route(path, http_method.to(handler)),
        )
        .await;

        // Set URI
        let req = match uri_to_call {
            Some(uri) => test_req.uri(uri),
            None => test_req,
        };

        // Set the JSON body if provided
        let req = match recipe_req {
            Some(ref _r) => req.set_json(recipe_req.unwrap()),
            None => req,
        };

        // Call the service and get the response
        let response = test::call_service(&app, req.to_request()).await;

        // Extract the HttpResponse from the ServiceResponse
        response.into_parts().1
    }

    /// execute a test request
    async fn execute<F, Args, R, Ret>(
        repo: &R,
        path: &str,
        uri_to_call: Option<&str>,
        http_method: Route,
        test_req: TestRequest,
        handler: F,
        recipe_req: Option<impl Serialize>,
    ) -> Ret
    where
        R: TranslationRepository,
        F: Handler<Args>,
        Args: FromRequest + 'static,
        F::Output: Responder,
        Ret: for<'de> Deserialize<'de>,
    {
        // init service
        let app = test::init_service(
            App::new()
                .app_data(Data::new(repo.clone()))
                .route(path, http_method.to(handler)),
        )
        .await;

        // set uri
        let req = match uri_to_call {
            Some(uri) => test_req.uri(uri),
            None => test_req,
        };

        // Set json body
        let req = match recipe_req {
            Some(ref _r) => req.set_json(recipe_req.unwrap()),
            None => req,
        };

        test::call_and_read_body_json(&app, req.to_request()).await
    }

    fn assert_on_translation_response(
        actual: &TranslationResponse,
        expected: &TranslationRecord,
        check_id: bool,
    ) {
        let (id, word, lang, translations, translation_lang) = expected.flat();

        if check_id {
            assert_eq!(&actual.id, id);
        }
        assert_eq!(&actual.word, word);
        assert_eq!(&actual.lang, lang);
        assert_on_translations(&actual.translations, &translations);
        assert_eq!(&actual.translation_lang, translation_lang);
    }
}
