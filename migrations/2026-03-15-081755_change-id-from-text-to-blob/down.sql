PRAGMA foreign_keys=OFF;

-- Create mapping tables
CREATE TEMPORARY TABLE category_uuid_map (
                                           old_id BLOB PRIMARY KEY,
                                           new_id TEXT NOT NULL
);

CREATE TEMPORARY TABLE drive_uuid_map (
                                        old_id BLOB PRIMARY KEY,
                                        new_id TEXT NOT NULL
);

-- Convert BLOB → TEXT hex
INSERT INTO category_uuid_map (old_id, new_id)
SELECT id, lower(hex(id)) FROM file_categories;

INSERT INTO drive_uuid_map (old_id, new_id)
SELECT id, lower(hex(id)) FROM drive_entries;

-- Create new tables with TEXT IDs
CREATE TABLE file_categories_old (
                                     id   TEXT PRIMARY KEY NOT NULL,
                                     name TEXT NOT NULL
);

CREATE TABLE drive_entries_old (
                                   id              TEXT PRIMARY KEY NOT NULL,
                                   category_id     TEXT NOT NULL REFERENCES file_categories_old(id),
                                   name            TEXT NOT NULL,
                                   available_space BIGINT NOT NULL DEFAULT 0,
                                   insertion_time  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE file_entries_old (
                                  id       TEXT PRIMARY KEY NOT NULL,
                                  drive_id TEXT NOT NULL REFERENCES drive_entries_old(id) ON DELETE CASCADE,
                                  path     TEXT COLLATE NOCASE NOT NULL,
                                  weight   BIGINT NOT NULL
);

-- Copy data back
INSERT INTO file_categories_old (id, name)
SELECT cim.new_id, fc.name
FROM file_categories fc
         JOIN category_uuid_map cim ON fc.id = cim.old_id;

INSERT INTO drive_entries_old (id, category_id, name, available_space, insertion_time)
SELECT dim.new_id, cim.new_id, de.name, de.available_space, de.insertion_time
FROM drive_entries de
         JOIN drive_uuid_map dim ON de.id = dim.old_id
         JOIN category_uuid_map cim ON de.category_id = cim.old_id;

INSERT INTO file_entries_old (id, drive_id, path, weight)
SELECT hex(randomblob(16)), dim.new_id, fe.path, fe.weight
FROM file_entries fe
         JOIN drive_uuid_map dim ON fe.drive_id = dim.old_id;

-- Drop BLOB tables and rename
DROP TABLE file_entries;
DROP TABLE drive_entries;
DROP TABLE file_categories;

ALTER TABLE file_categories_old RENAME TO file_categories;
ALTER TABLE drive_entries_old RENAME TO drive_entries;
ALTER TABLE file_entries_old RENAME TO file_entries;

PRAGMA foreign_keys=ON;