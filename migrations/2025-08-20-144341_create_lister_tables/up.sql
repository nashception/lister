CREATE TABLE file_categories
(
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL
);

CREATE TABLE drive_entries
(
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL
);

CREATE TABLE file_entries
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    categoryId INTEGER NOT NULL,
    driveId    INTEGER NOT NULL,
    path       TEXT    NOT NULL,
    weight     BIGINT  NOT NULL,
    FOREIGN KEY (categoryId) REFERENCES file_categories (id),
    FOREIGN KEY (driveId) REFERENCES drive_entries (id)
);