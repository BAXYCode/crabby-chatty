-- Add up migration script here
ALTER TABLE valid.refresh
     ADD COLUMN IF NOT EXISTS version INT8 DEFAULT 1  NOT NULL;
