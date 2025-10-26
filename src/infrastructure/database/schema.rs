use diesel::{allow_tables_to_appear_in_same_query, joinable, table};

table! {
    file_categories (id) {
        id -> Text,
        name -> Text,
    }
}

table! {
    drive_entries (id) {
        id -> Text,
        category_id -> Text,
        name -> Text,
        available_space -> BigInt,
        insertion_time -> Timestamp,
    }
}

table! {
    file_entries (id) {
        id -> Text,
        drive_id -> Text,
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
