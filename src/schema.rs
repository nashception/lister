diesel::table! {
    file_categories (Id) {
        Id -> Integer,
        Name -> Text,
    }
}

diesel::table! {
    drive_entries (Id) {
        Id -> Integer,
        Name -> Text,
    }
}

diesel::table! {
    file_entries (Id) {
        Id -> Integer,
        CategoryId -> Integer,
        DriveId -> Integer,
        Path -> Text,
        Weight -> BigInt,
    }
}

diesel::joinable!(file_entries -> file_categories (CategoryId));
diesel::joinable!(file_entries -> drive_entries (DriveId));
diesel::allow_tables_to_appear_in_same_query!(file_categories, file_entries);
diesel::allow_tables_to_appear_in_same_query!(drive_entries, file_entries);