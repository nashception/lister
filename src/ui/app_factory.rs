use crate::domain::entities::language::Language;
use crate::domain::ports::primary::file_indexing_use_case::FileIndexingUseCase;
use crate::domain::ports::primary::file_query_use_case::FileQueryUseCase;
use crate::domain::ports::primary::language_use_case::LanguageManagementUseCase;
use crate::domain::ports::secondary::directory_picker::DirectoryPicker;
use crate::domain::services::file_indexing_service::FileIndexingService;
use crate::domain::services::file_query_service::FileQueryService;
use crate::domain::services::language_service::LanguageService;
use crate::infrastructure::database::sqlite_repository::SqliteFileRepository;
use crate::infrastructure::filesystem::native_directory_picker::NativeDirectoryPicker;
use crate::infrastructure::i18n::json_translation_loader::JsonTranslationLoader;
use crate::utils::dialogs::popup_error_and_exit;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ListerAppService {
    pub query_use_case: Arc<dyn FileQueryUseCase>,
    pub indexing_use_case: Arc<dyn FileIndexingUseCase>,
    pub language_use_case: Arc<dyn LanguageManagementUseCase>,
    pub directory_picker: Arc<dyn DirectoryPicker>,
}

impl ListerAppService {
    #[must_use]
    pub fn create() -> Self {
        // Create the single repository instance
        let repository = Arc::new(
            SqliteFileRepository::new("app.db").unwrap_or_else(|error| popup_error_and_exit(error)),
        );
        let translation_loader = Arc::new(JsonTranslationLoader);
        let directory_picker = Arc::new(NativeDirectoryPicker);

        let query_service = Arc::new(FileQueryService::new(repository.clone()));
        let indexing_service = Arc::new(FileIndexingService::new(repository.clone()));
        let language_service =
            Arc::new(LanguageService::new(repository, translation_loader));

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
