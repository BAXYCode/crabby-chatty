-- Add up migration script here
CREATE TABLE IF NOT EXISTS valid.keypairs(
 paserk STRING(255) PRIMARY KEY NOT NULL,
 secret BYTEA NOT NULL,
 public BYTEA NOT NULL
);
