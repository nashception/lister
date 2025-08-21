CREATE TABLE file_categories
(
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL
);

CREATE TABLE drive_entries
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL,
    name        TEXT    NOT NULL,
    FOREIGN KEY (category_id) REFERENCES file_categories (id)
);

CREATE TABLE file_entries
(
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    drive_id INTEGER NOT NULL,
    path     TEXT    NOT NULL COLLATE NOCASE,
    weight   BIGINT  NOT NULL,
    FOREIGN KEY (drive_id) REFERENCES drive_entries (id)
);