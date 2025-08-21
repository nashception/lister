diesel::table! {
    file_categories (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    drive_entries (id) {
        id -> Integer,
        category_id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    file_entries (id) {
        id -> Integer,
        drive_id -> Integer,
        path -> Text,
        weight -> BigInt,
    }
}

diesel::joinable!(drive_entries -> file_categories (category_id));
diesel::joinable!(file_entries -> drive_entries (drive_id));
diesel::allow_tables_to_appear_in_same_query!(file_categories, drive_entries);
diesel::allow_tables_to_appear_in_same_query!(drive_entries, file_entries);