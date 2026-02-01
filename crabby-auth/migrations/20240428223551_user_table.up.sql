-- Postgres-compatible version of the original CockroachDB migration.

CREATE SCHEMA IF NOT EXISTS valid;

-- Needed for gen_random_uuid()
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS valid.username (
    id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    username VARCHAR(255) NOT NULL
);
-- Cockroach used "ADD CONSTRAINT IF NOT EXISTS ... UNIQUE".
-- In Postgres, a unique index is the most portable "IF NOT EXISTS" equivalent.
CREATE UNIQUE INDEX IF NOT EXISTS username_username_unique ON valid.username (username);

CREATE TABLE IF NOT EXISTS valid.password (
    id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    password VARCHAR(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS valid.email (
    id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    email VARCHAR(255) NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS email_email_unique ON valid.email (email);

CREATE TABLE IF NOT EXISTS valid.last_login (
    id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    last_login TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
);

CREATE TABLE IF NOT EXISTS valid.users (
    user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email_id BIGINT NOT NULL,
    username_id BIGINT NOT NULL,
    password_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    last_login_id BIGINT NOT NULL,
    is_admin BOOLEAN NOT NULL
);

CREATE INDEX IF NOT EXISTS users_username_index ON valid.username (username);
CREATE INDEX IF NOT EXISTS user_id_index ON valid.users (user_id);

CREATE TABLE IF NOT EXISTS valid.ip (
    id BIGINT PRIMARY KEY NOT NULL,
    ip VARCHAR(255) NOT NULL,
    user_id UUID NOT NULL
);

ALTER TABLE valid.users
    ADD CONSTRAINT users_username_foreign FOREIGN KEY (username_id) REFERENCES valid.username (id);

ALTER TABLE valid.users
    ADD CONSTRAINT users_password_foreign FOREIGN KEY (password_id) REFERENCES valid.password (id);

ALTER TABLE valid.users
    ADD CONSTRAINT users_email_foreign FOREIGN KEY (email_id) REFERENCES valid.email (id);

ALTER TABLE valid.users
    ADD CONSTRAINT users_last_login_foreign FOREIGN KEY (last_login_id) REFERENCES valid.last_login (id);

ALTER TABLE valid.ip
    ADD CONSTRAINT ips_userid_foreign FOREIGN KEY (user_id) REFERENCES valid.users (user_id);
