-- Postgres-compatible version of the original CockroachDB migration.
ALTER TABLE valid.refresh
    ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1;
