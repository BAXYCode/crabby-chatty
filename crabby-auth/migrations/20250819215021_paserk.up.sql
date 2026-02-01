-- Postgres-compatible version of the original CockroachDB migration.
CREATE TABLE IF NOT EXISTS valid.keypairs (
    paserk VARCHAR(255) PRIMARY KEY NOT NULL,
    secret BYTEA NOT NULL,
    public BYTEA NOT NULL
);
