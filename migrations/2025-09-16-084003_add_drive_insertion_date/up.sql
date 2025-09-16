ALTER TABLE drive_entries
    ADD COLUMN insertion_time TIMESTAMP NOT NULL DEFAULT '2025-09-16 08:40:03';

-- noinspection SqlWithoutWhere
UPDATE drive_entries
SET insertion_time = CURRENT_TIMESTAMP;