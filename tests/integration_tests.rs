use lister::domain::entities::file_entry::FileEntry;
use lister::domain::entities::language::Language;
use lister::domain::ports::primary::file_indexing_use_case::FileIndexingUseCase;
use lister::domain::ports::primary::file_query_use_case::FileQueryUseCase;
use lister::domain::ports::primary::language_use_case::LanguageManagementUseCase;
use lister::domain::services::file_indexing_service::FileIndexingService;
use lister::domain::services::file_query_service::FileQueryService;
use lister::domain::services::language_service::LanguageService;
use lister::infrastructure::database::sqlite_repository::SqliteFileRepository;
use lister::infrastructure::i18n::json_translation_loader::JsonTranslationLoader;
use std::sync::Arc;
use tempfile::TempDir;

// Test helpers and fixtures
struct TestFixture {
    _temp_dir: TempDir, // Store it to prevent its disposal
    query_service: Arc<dyn FileQueryUseCase>,
    indexing_service: Arc<dyn FileIndexingUseCase>,
    language_service: Arc<dyn LanguageManagementUseCase>,
}

impl TestFixture {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}", db_path.display());

        let repo =
            Arc::new(SqliteFileRepository::new(&db_url).expect("Failed to create test database"));

        let query_service = Arc::new(FileQueryService::new(repo.clone()));
        let indexing_service = Arc::new(FileIndexingService::new(repo.clone()));
        let translation_loader = Arc::new(JsonTranslationLoader);
        let language_service = Arc::new(LanguageService::new(repo.clone(), translation_loader));

        Self {
            _temp_dir: temp_dir,
            query_service,
            indexing_service,
            language_service,
        }
    }

    fn create_test_files(&self) -> Vec<FileEntry> {
        vec![
            FileEntry {
                path: "documents/report.pdf".to_string(),
                size_bytes: 1024,
            },
            FileEntry {
                path: "images/photo.jpg".to_string(),
                size_bytes: 2048,
            },
            FileEntry {
                path: "code/main.rs".to_string(),
                size_bytes: 512,
            },
            FileEntry {
                path: "documents/invoice.pdf".to_string(),
                size_bytes: 768,
            },
        ]
    }
}

#[tokio::test]
async fn test_complete_file_indexing_workflow() {
    let fixture = TestFixture::new();
    let files = fixture.create_test_files();

    // Test indexing workflow
    let result = fixture
        .indexing_service
        .insert_in_database("Work".to_string(), "Laptop".to_string(), files.clone())
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 4);

    // Verify files were indexed
    let query_result = fixture.query_service.list_files(0, 10).await;
    assert!(query_result.is_ok());

    let paginated = query_result.unwrap();
    assert_eq!(paginated.total_count, 4);
    assert_eq!(paginated.items.len(), 4);

    // Verify file metadata
    let first_file = &paginated.items[0];
    assert_eq!(first_file.category_name, "Work");
    assert_eq!(first_file.drive_name, "Laptop");
    assert!(files.iter().any(|f| f.path == first_file.path));
}

#[tokio::test]
async fn test_duplicate_removal_workflow() {
    let fixture = TestFixture::new();
    let files = fixture.create_test_files();

    // Index files first time
    fixture
        .indexing_service
        .insert_in_database("Work".to_string(), "Laptop".to_string(), files.clone())
        .await
        .expect("First indexing failed");

    // Remove duplicates
    let remove_result = fixture
        .indexing_service
        .remove_duplicates("Work".to_string(), "Laptop".to_string())
        .await;
    assert!(remove_result.is_ok());

    // Verify files were removed
    let query_result = fixture.query_service.list_files(0, 10).await.unwrap();
    assert_eq!(query_result.total_count, 0);

    // Index again after removal
    fixture
        .indexing_service
        .insert_in_database("Work".to_string(), "Laptop".to_string(), files)
        .await
        .expect("Second indexing failed");

    let final_result = fixture.query_service.list_files(0, 10).await.unwrap();
    assert_eq!(final_result.total_count, 4);
}

#[tokio::test]
async fn test_file_search_functionality() {
    let fixture = TestFixture::new();
    let files = fixture.create_test_files();

    // Index test files
    fixture
        .indexing_service
        .insert_in_database("Work".to_string(), "Laptop".to_string(), files)
        .await
        .expect("Indexing failed");

    // Test search by extension
    let pdf_results = fixture
        .query_service
        .search_files(".pdf", 0, 10)
        .await
        .unwrap();
    assert_eq!(pdf_results.total_count, 2);
    assert!(pdf_results.items.iter().all(|f| f.path.contains(".pdf")));

    // Test search by directory
    let doc_results = fixture
        .query_service
        .search_files("documents", 0, 10)
        .await
        .unwrap();
    assert_eq!(doc_results.total_count, 2);
    assert!(
        doc_results
            .items
            .iter()
            .all(|f| f.path.contains("documents"))
    );

    // Test search by filename
    let main_results = fixture
        .query_service
        .search_files("main", 0, 10)
        .await
        .unwrap();
    assert_eq!(main_results.total_count, 1);
    assert_eq!(main_results.items[0].path, "code/main.rs");

    // Test empty search returns all files
    let all_results = fixture.query_service.search_files("", 0, 10).await.unwrap();
    assert_eq!(all_results.total_count, 4);
}

#[tokio::test]
async fn test_pagination_behavior() {
    let fixture = TestFixture::new();

    // Create many files to test pagination
    let mut many_files = Vec::new();
    for i in 0..250 {
        many_files.push(FileEntry {
            path: format!("file_{:03}.txt", i),
            size_bytes: i as i64 * 10,
        });
    }

    fixture
        .indexing_service
        .insert_in_database("Test".to_string(), "Drive".to_string(), many_files)
        .await
        .expect("Indexing failed");

    // Test first page
    let page_0 = fixture.query_service.list_files(0, 100).await.unwrap();
    assert_eq!(page_0.items.len(), 100);
    assert_eq!(page_0.total_count, 250);

    // Test second page
    let page_1 = fixture.query_service.list_files(1, 100).await.unwrap();
    assert_eq!(page_1.items.len(), 100);
    assert_eq!(page_1.total_count, 250);

    // Test last page
    let page_2 = fixture.query_service.list_files(2, 100).await.unwrap();
    assert_eq!(page_2.items.len(), 50);
    assert_eq!(page_2.total_count, 250);

    // Test beyond last page
    let page_3 = fixture.query_service.list_files(3, 100).await.unwrap();
    assert_eq!(page_3.items.len(), 0);
    assert_eq!(page_3.total_count, 250);
}

#[tokio::test]
async fn test_search_result_count_accuracy() {
    let fixture = TestFixture::new();
    let files = fixture.create_test_files();

    fixture
        .indexing_service
        .insert_in_database("Work".to_string(), "Laptop".to_string(), files)
        .await
        .expect("Indexing failed");

    // Test that search results total_count is accurate
    let search_query = ".pdf";
    let search_results = fixture
        .query_service
        .search_files(search_query, 0, 100)
        .await
        .unwrap();

    // Should find exactly 2 PDF files
    assert_eq!(search_results.total_count, 2);
    assert_eq!(search_results.items.len(), 2);

    // Verify the count matches the actual items returned
    assert!(search_results.items.iter().all(|f| f.path.contains(".pdf")));
}

#[tokio::test]
async fn test_language_management_workflow() {
    let fixture = TestFixture::new();

    // Test default language
    let default_lang = fixture.language_service.get_current_language().unwrap();
    assert_eq!(default_lang.code(), "en");

    // Test language change
    let french = Language::french();
    fixture
        .language_service
        .set_language(french.clone())
        .expect("Failed to set language");

    let current_lang = fixture.language_service.get_current_language().unwrap();
    assert_eq!(current_lang.code(), "fr");

    // Test translation loading
    let translations = fixture
        .language_service
        .load_translations(&current_lang)
        .unwrap();
    assert!(!translations.is_empty());

    // Test language toggle
    let toggled = current_lang.toggle();
    assert_eq!(toggled.code(), "en");
}

#[tokio::test]
async fn test_multiple_categories_and_drives() {
    let fixture = TestFixture::new();
    let files = fixture.create_test_files();

    // Index files in different categories and drives
    fixture
        .indexing_service
        .insert_in_database("Work".to_string(), "Laptop".to_string(), files.clone())
        .await
        .expect("First indexing failed");

    fixture
        .indexing_service
        .insert_in_database("Personal".to_string(), "Desktop".to_string(), files.clone())
        .await
        .expect("Second indexing failed");

    fixture
        .indexing_service
        .insert_in_database("Work".to_string(), "Server".to_string(), files)
        .await
        .expect("Third indexing failed");

    // Verify all files are indexed
    let all_files = fixture.query_service.list_files(0, 20).await.unwrap();
    assert_eq!(all_files.total_count, 12); // 4 files × 3 locations

    // Verify different categories exist
    let categories: std::collections::HashSet<_> = all_files
        .items
        .iter()
        .map(|f| f.category_name.clone())
        .collect();
    assert!(categories.contains("Work"));
    assert!(categories.contains("Personal"));

    // Verify different drives exist
    let drives: std::collections::HashSet<_> = all_files
        .items
        .iter()
        .map(|f| f.drive_name.clone())
        .collect();
    assert!(drives.contains("Laptop"));
    assert!(drives.contains("Desktop"));
    assert!(drives.contains("Server"));
}

#[tokio::test]
async fn test_concurrent_operations() {
    let fixture = TestFixture::new();
    let files = fixture.create_test_files();

    // Index initial files
    fixture
        .indexing_service
        .insert_in_database("Work".to_string(), "Laptop".to_string(), files)
        .await
        .expect("Initial indexing failed");

    // Run concurrent read operations
    let query_service = fixture.query_service.clone();
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let service = query_service.clone();
            tokio::spawn(async move {
                let result = service.list_files(0, 10).await;
                (i, result)
            })
        })
        .collect();

    // Wait for all operations to complete
    for handle in handles {
        let (_, result) = handle.await.expect("Task failed");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().total_count, 4);
    }
}

#[tokio::test]
async fn test_edge_cases_and_error_handling() {
    let fixture = TestFixture::new();

    // Test empty search
    let empty_result = fixture.query_service.search_files("", 0, 10).await.unwrap();
    assert_eq!(empty_result.total_count, 0);

    // Test search with no matches
    let no_matches = fixture
        .query_service
        .search_files("nonexistent", 0, 10)
        .await
        .unwrap();
    assert_eq!(no_matches.total_count, 0);

    // Test pagination with no data
    let no_data = fixture.query_service.list_files(5, 10).await.unwrap();
    assert_eq!(no_data.total_count, 0);
    assert_eq!(no_data.items.len(), 0);

    // Test remove duplicates with no data
    let remove_empty = fixture
        .indexing_service
        .remove_duplicates("NonExistent".to_string(), "Drive".to_string())
        .await;
    assert!(remove_empty.is_ok());

    // Test indexing empty file list
    let empty_index = fixture
        .indexing_service
        .insert_in_database("Empty".to_string(), "Drive".to_string(), vec![])
        .await;
    assert!(empty_index.is_ok());
    assert_eq!(empty_index.unwrap(), 0);
}

// Benchmark test to ensure performance doesn't regress
#[tokio::test]
async fn test_search_performance_with_large_dataset() {
    let fixture = TestFixture::new();

    // Create a large dataset
    let mut large_dataset = Vec::new();
    for i in 0..10000 {
        large_dataset.push(FileEntry {
            path: format!(
                "category_{}/subcategory_{}/file_{:05}.txt",
                i % 10,
                i % 100,
                i
            ),
            size_bytes: i as i64,
        });
    }

    fixture
        .indexing_service
        .insert_in_database("Large".to_string(), "Dataset".to_string(), large_dataset)
        .await
        .expect("Large dataset indexing failed");

    let start = std::time::Instant::now();

    // Test search performance
    let search_result = fixture
        .query_service
        .search_files("category_5", 0, 100)
        .await
        .unwrap();

    let elapsed = start.elapsed();

    // Ensure reasonable performance (adjust threshold as needed)
    assert!(
        elapsed.as_millis() < 1000,
        "Search took too long: {:?}",
        elapsed
    );
    assert!(search_result.total_count > 0);

    // Test pagination performance
    let start = std::time::Instant::now();
    let page_result = fixture.query_service.list_files(50, 100).await.unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 500,
        "Pagination took too long: {:?}",
        elapsed
    );
    assert_eq!(page_result.items.len(), 100);
}
