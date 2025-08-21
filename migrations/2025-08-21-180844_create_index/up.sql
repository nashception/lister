-- Critical indexes for JOIN performance
CREATE INDEX IF NOT EXISTS idx_drive_entries_category_id ON drive_entries(category_id);
CREATE INDEX IF NOT EXISTS idx_file_entries_drive_id ON file_entries(drive_id);

-- Search performance index
CREATE INDEX IF NOT EXISTS idx_file_entries_path ON file_entries(path);

-- Composite index for main queries
CREATE INDEX IF NOT EXISTS idx_file_entries_drive_path ON file_entries(drive_id, path);

-- Analyze tables after creating indexes
ANALYZE;