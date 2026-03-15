PRAGMA foreign_keys=OFF;

-- Step 1: Temporary mapping tables
CREATE TEMPORARY TABLE category_uuid_text_to_blob_map (
    old_id TEXT PRIMARY KEY,
    new_id BLOB NOT NULL
);

CREATE TEMPORARY TABLE drive_uuid_text_to_blob_map (
    old_id TEXT PRIMARY KEY,
    new_id BLOB NOT NULL
);

-- Step 2: Generate 16-byte UUIDs for existing records
INSERT INTO category_uuid_text_to_blob_map (old_id, new_id)
SELECT id, randomblob(16) FROM file_categories;

INSERT INTO drive_uuid_text_to_blob_map (old_id, new_id)
SELECT id, randomblob(16) FROM drive_entries;

-- Step 3: Create new tables with BLOB IDs
CREATE TABLE file_categories_new (
                                     id   BLOB PRIMARY KEY NOT NULL,
                                     name TEXT NOT NULL
);

CREATE TABLE drive_entries_new (
                                   id              BLOB PRIMARY KEY NOT NULL,
                                   category_id     BLOB NOT NULL REFERENCES file_categories_new(id),
                                   name            TEXT NOT NULL,
                                   available_space BIGINT NOT NULL DEFAULT 0,
                                   insertion_time  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE file_entries_new (
                                  id       BLOB PRIMARY KEY NOT NULL,
                                  drive_id BLOB NOT NULL REFERENCES drive_entries_new(id) ON DELETE CASCADE,
                                  path     TEXT COLLATE NOCASE NOT NULL,
                                  weight   BIGINT NOT NULL
);

-- Step 4: Copy data using the new BLOB UUIDs
INSERT INTO file_categories_new (id, name)
SELECT cim.new_id, fc.name
FROM file_categories fc
         JOIN category_uuid_text_to_blob_map cim ON fc.id = cim.old_id;

INSERT INTO drive_entries_new (id, category_id, name, available_space, insertion_time)
SELECT dim.new_id, cim.new_id, de.name, de.available_space, de.insertion_time
FROM drive_entries de
         JOIN drive_uuid_text_to_blob_map dim ON de.id = dim.old_id
         JOIN category_uuid_text_to_blob_map cim ON de.category_id = cim.old_id;

INSERT INTO file_entries_new (id, drive_id, path, weight)
SELECT randomblob(16), dim.new_id, fe.path, fe.weight
FROM file_entries fe
         JOIN drive_uuid_text_to_blob_map dim ON fe.drive_id = dim.old_id;

-- Step 5: Drop old tables and rename
DROP TABLE file_entries;
DROP TABLE drive_entries;
DROP TABLE file_categories;

ALTER TABLE file_categories_new RENAME TO file_categories;
ALTER TABLE drive_entries_new RENAME TO drive_entries;
ALTER TABLE file_entries_new RENAME TO file_entries;

PRAGMA foreign_keys=ON;

-- Critical indexes for JOIN performance
CREATE INDEX IF NOT EXISTS idx_drive_entries_category_id ON drive_entries (category_id);
CREATE INDEX IF NOT EXISTS idx_file_entries_drive_id ON file_entries (drive_id);

-- Analyze tables after creating indexes
ANALYZE;