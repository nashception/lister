PRAGMA foreign_keys= OFF;

CREATE TABLE file_entries_old
(
    id       TEXT                NOT NULL PRIMARY KEY,
    drive_id TEXT                NOT NULL
        REFERENCES drive_entries,
    path     TEXT COLLATE NOCASE NOT NULL,
    weight   BIGINT              NOT NULL
);

INSERT INTO file_entries_old (id, drive_id, path, weight)
SELECT id, drive_id, path, weight
FROM file_entries;

DROP TABLE file_entries;

ALTER TABLE file_entries_old
    RENAME TO file_entries;

PRAGMA foreign_keys= ON;