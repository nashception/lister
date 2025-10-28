use crate::application::file_indexing_service::FileIndexingService;
use crate::application::file_query_service::FileQueryService;
use crate::application::language_service::LanguageService;
use crate::domain::model::language::Language;
use crate::infrastructure::database::command_repository::CommandRepository;
use crate::infrastructure::database::language_repository::LanguageRepository;
use crate::infrastructure::database::pool::SqliteRepositoryPool;
use crate::infrastructure::database::query_repository::QueryRepository;
use crate::infrastructure::filesystem::native_directory_picker::NativeDirectoryPicker;
use crate::infrastructure::i18n::json_translation_loader::JsonTranslationLoader;
use crate::utils::dialogs::popup_error_and_exit;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ListerAppService {
    pub query_use_case: Arc<FileQueryService>,
    pub indexing_use_case: Arc<FileIndexingService>,
    pub language_use_case: Arc<LanguageService>,
    pub directory_picker: Arc<NativeDirectoryPicker>,
}

impl ListerAppService {
    #[must_use]
    pub fn create() -> Self {
        let directory_picker = Arc::new(NativeDirectoryPicker);

        let pool =
            SqliteRepositoryPool::new("app.db").unwrap_or_else(|error| popup_error_and_exit(error));

        let query_repository = QueryRepository::new(Arc::clone(&pool));
        let command_repository = CommandRepository::new(Arc::clone(&pool));
        let language_repository = LanguageRepository::new(pool);

        let query_service = Arc::new(FileQueryService::new(query_repository));
        let indexing_service = Arc::new(FileIndexingService::new(command_repository));
        let language_service = Arc::new(LanguageService::new(
            language_repository,
            JsonTranslationLoader,
        ));

        Self {
            query_use_case: query_service,
            indexing_use_case: indexing_service,
            language_use_case: language_service,
            directory_picker,
        }
    }

    #[must_use]
    pub fn translations(&self) -> (Language, HashMap<String, String>) {
        let current_language = self
            .language_use_case
            .get_current_language()
            .unwrap_or(Language::English);
        let translations = self
            .language_use_case
            .load_translations(&current_language)
            .unwrap_or_default();

        (current_language, translations)
    }
}
