-- Add up migration script here
DROP TABLE IF EXISTS valid.keypairs;

CREATE TABLE IF NOT EXISTS valid.refresh_metadata (
  userId UUID NOT NULL,
  id UUID PRIMARY KEY,
  token_hash  STRING(255) NOT NULL,
  iat TIMESTAMPTZ NOT NULL,
  nbf TIMESTAMPTZ NOT NULL,
  exp TIMESTAMPTZ NOT NULL
);

ALTER TABLE valid.refresh_metadata
ADD CONSTRAINT user_id_foreign_to_refresh_data FOREIGN KEY (userId) REFERENCES valid.users (user_id)
