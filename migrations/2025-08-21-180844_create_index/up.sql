-- Critical indexes for JOIN performance
CREATE INDEX IF NOT EXISTS idx_drive_entries_category_id ON drive_entries(category_id);
CREATE INDEX IF NOT EXISTS idx_file_entries_drive_id ON file_entries(drive_id);

-- Analyze tables after creating indexes
ANALYZE;