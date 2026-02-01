-- Add down migration script here
ALTER TABLE valid.refresh
     DROP COLUMN IF EXISTS version;
