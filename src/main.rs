fn main() {
    println!("Hello, world!");
}

#[test]
fn lister_repository_test_find_all_categories() {
    let lister_repository = InMemoryListerRepository::new();

    let categories: Vec<FileCategory> = lister_repository.find_all_categories().cloned().collect();

    assert_eq!(categories, InMemoryListerRepository::dummy_categories());
}

#[test]
fn lister_repository_test_find_all_files_by_category_id() {
    fn expected_files_for_category(id: u64) -> Vec<FileEntry> {
        InMemoryListerRepository::dummy_files()
            .into_iter()
            .filter(|f| f.category_id == id)
            .collect()
    }

    let repo = InMemoryListerRepository::new();

    for category_id in [0, 1] {
        let actual: Vec<FileEntry> = repo
            .find_all_files_by_category_id(category_id)
            .cloned()
            .collect();
        let expected = expected_files_for_category(category_id);
        assert_eq!(actual, expected, "Mismatch for category {}", category_id);
    }
}

#[derive(Clone, Debug, PartialEq)]
struct FileCategory {
    id: u64,
    name: String,
}

#[derive(Clone, Debug, PartialEq)]
struct FileEntry {
    category_id: u64,
    path: String,
    weight: u64,
}

trait ListerRepository {
    fn find_all_categories(&self) -> impl Iterator<Item = &FileCategory>;
    fn find_all_files_by_category_id(&self, group_id: u64) -> impl Iterator<Item = &FileEntry>;
    fn find_all_files(&self) -> impl Iterator<Item = &FileEntry>;
}

struct InMemoryListerRepository {
    categories: Vec<FileCategory>,
    files: Vec<FileEntry>,
}

impl InMemoryListerRepository {
    fn dummy_categories() -> Vec<FileCategory> {
        vec![
            FileCategory {
                id: 0,
                name: String::from("Series"),
            },
            FileCategory {
                id: 1,
                name: String::from("Movies"),
            },
        ]
    }
    fn dummy_files() -> Vec<FileEntry> {
        vec![
            FileEntry {
                category_id: 0,
                path: String::from("Dr House/Season 1/Episode 1.mkv"),
                weight: 2000000,
            },
            FileEntry {
                category_id: 0,
                path: String::from("Dr House/Season 1/Episode 2.mkv"),
                weight: 2500000,
            },
            FileEntry {
                category_id: 0,
                path: String::from("Dr House/Season 1/Episode 3.mkv"),
                weight: 3000000,
            },
            FileEntry {
                category_id: 1,
                path: String::from("Venom/Venom 1.mkv"),
                weight: 54000000,
            },
            FileEntry {
                category_id: 1,
                path: String::from("Venom/Venom 2.mkv"),
                weight: 65000000,
            },
        ]
    }
    fn new() -> Self {
        Self {
            categories: Self::dummy_categories(),
            files: Self::dummy_files(),
        }
    }
}

impl ListerRepository for InMemoryListerRepository {
    fn find_all_categories(&self) -> impl Iterator<Item = &FileCategory> {
        self.categories.iter()
    }

    fn find_all_files_by_category_id(&self, group_id: u64) -> impl Iterator<Item = &FileEntry> {
        self.files
            .iter()
            .filter(move |entry| entry.category_id == group_id)
    }

    fn find_all_files(&self) -> impl Iterator<Item = &FileEntry> {
        self.files.iter()
    }
}
