-- Migration 1: Remove duplicates

-- Step 1: Handle file_categories duplicates
-- Keep the lowest id for each unique name, update references, then delete duplicates

UPDATE drive_entries
SET category_id = (
    SELECT MIN(id)
    FROM file_categories fc2
    WHERE fc2.name = (
        SELECT name
        FROM file_categories fc3
        WHERE fc3.id = drive_entries.category_id
    )
)
WHERE category_id NOT IN (
    SELECT MIN(id)
    FROM file_categories
    GROUP BY name
);

DELETE FROM file_categories
WHERE id NOT IN (
    SELECT MIN(id)
    FROM file_categories
    GROUP BY name
);

-- Step 2: Handle drive_entries duplicates
-- Keep the lowest id for each unique (name, category_id) combination

UPDATE file_entries
SET drive_id = (
    SELECT MIN(de2.id)
    FROM drive_entries de2
             INNER JOIN drive_entries de3 ON de3.id = file_entries.drive_id
    WHERE de2.name = de3.name AND de2.category_id = de3.category_id
)
WHERE drive_id NOT IN (
    SELECT MIN(id)
    FROM drive_entries
    GROUP BY name, category_id
);

-- Step 3: Handle file_entries duplicates that may exist after drive consolidation
DELETE FROM file_entries
WHERE id NOT IN (
    SELECT MIN(id)
    FROM file_entries
    GROUP BY drive_id, path
);

DELETE FROM drive_entries
WHERE id NOT IN (
    SELECT MIN(id)
    FROM drive_entries
    GROUP BY name, category_id
);


-- ============================================================================
-- Migration 2: Convert to UUID primary keys

-- Step 1: Create temporary mapping tables for ID conversion
CREATE TEMPORARY TABLE category_id_map (
                                           old_id INTEGER PRIMARY KEY,
                                           new_id TEXT NOT NULL
);

CREATE TEMPORARY TABLE drive_id_map (
                                        old_id INTEGER PRIMARY KEY,
                                        new_id TEXT NOT NULL
);

-- Step 2: Generate UUID mappings for existing records
INSERT INTO category_id_map (old_id, new_id)
SELECT
    id,
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(6)))
FROM file_categories;

INSERT INTO drive_id_map (old_id, new_id)
SELECT
    id,
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(6)))
FROM drive_entries;

-- Step 3: Create new tables with UUID primary keys (id as first column)
CREATE TABLE file_categories_new (
                                     id TEXT PRIMARY KEY NOT NULL,
                                     name TEXT NOT NULL
);

CREATE TABLE drive_entries_new (
                                   id TEXT PRIMARY KEY NOT NULL,
                                   category_id TEXT NOT NULL REFERENCES file_categories_new(id),
                                   name TEXT NOT NULL,
                                   available_space BIGINT DEFAULT 0 NOT NULL,
                                   insertion_time TIMESTAMP DEFAULT '2025-09-16 08:40:03' NOT NULL
);

CREATE TABLE file_entries_new (
                                  id TEXT PRIMARY KEY NOT NULL,
                                  drive_id TEXT NOT NULL REFERENCES drive_entries_new(id),
                                  path TEXT COLLATE NOCASE NOT NULL,
                                  weight BIGINT NOT NULL
);

-- Step 4: Migrate data using the UUID mappings
INSERT INTO file_categories_new (id, name)
SELECT
    cim.new_id,
    fc.name
FROM file_categories fc
         JOIN category_id_map cim ON fc.id = cim.old_id;

INSERT INTO drive_entries_new (id, category_id, name, available_space, insertion_time)
SELECT
    dim.new_id,
    cim.new_id,
    de.name,
    de.available_space,
    de.insertion_time
FROM drive_entries de
         JOIN drive_id_map dim ON de.id = dim.old_id
         JOIN category_id_map cim ON de.category_id = cim.old_id;

INSERT INTO file_entries_new (id, drive_id, path, weight)
SELECT
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(6))),
    dim.new_id,
    fe.path,
    fe.weight
FROM file_entries fe
         JOIN drive_id_map dim ON fe.drive_id = dim.old_id;

-- Step 5: Drop old tables and rename new ones
DROP TABLE file_entries;
DROP TABLE drive_entries;
DROP TABLE file_categories;

ALTER TABLE file_categories_new RENAME TO file_categories;
ALTER TABLE drive_entries_new RENAME TO drive_entries;
ALTER TABLE file_entries_new RENAME TO file_entries;
