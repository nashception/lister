use diesel::{allow_tables_to_appear_in_same_query, joinable, table};

table! {
    file_categories (id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
    drive_entries (id) {
        id -> Integer,
        category_id -> Integer,
        name -> Text,
        available_space -> BigInt,
    }
}

table! {
    file_entries (id) {
        id -> Integer,
        drive_id -> Integer,
        path -> Text,
        weight -> BigInt,
    }
}

table! {
    settings (key) {
        key -> Text,
        value -> Text,
    }
}

joinable!(drive_entries -> file_categories (category_id));
joinable!(file_entries -> drive_entries (drive_id));

allow_tables_to_appear_in_same_query!(file_categories, drive_entries, file_entries,);
