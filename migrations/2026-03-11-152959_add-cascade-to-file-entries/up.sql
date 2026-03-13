PRAGMA foreign_keys= OFF;

-- Create new table with cascade
CREATE TABLE file_entries_new
(
    id       TEXT                NOT NULL PRIMARY KEY,
    drive_id TEXT                NOT NULL,
    path     TEXT COLLATE NOCASE NOT NULL,
    weight   BIGINT              NOT NULL,
    FOREIGN KEY (drive_id)
        REFERENCES drive_entries (id)
        ON DELETE CASCADE
);

-- Copy existing data
INSERT INTO file_entries_new (id, drive_id, path, weight)
SELECT id, drive_id, path, weight
FROM file_entries;

-- Replace old table
DROP TABLE file_entries;

ALTER TABLE file_entries_new
    RENAME TO file_entries;

PRAGMA foreign_keys= ON;