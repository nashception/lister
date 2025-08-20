diesel::table! {
    file_categories (Id) {
        Id -> Integer,
        Name -> Text,
    }
}

diesel::table! {
    file_entries (Id) {
        Id -> Integer,
        CategoryId -> Integer,
        Path -> Text,
        Weight -> BigInt,
    }
}

diesel::joinable!(file_entries -> file_categories (CategoryId));
diesel::allow_tables_to_appear_in_same_query!(file_categories, file_entries);