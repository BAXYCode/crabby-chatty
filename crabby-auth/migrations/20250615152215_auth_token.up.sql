-- Postgres-compatible version of the original CockroachDB migration.

CREATE TABLE IF NOT EXISTS valid.tokens (
    user_id UUID NOT NULL PRIMARY KEY,
    refresh_id BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS valid.refresh (
    id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    refresh TEXT NOT NULL
);

ALTER TABLE valid.tokens
    ADD CONSTRAINT token_refresh_foreign FOREIGN KEY (refresh_id) REFERENCES valid.refresh (id);

-- NOTE: original file had a comment saying the next FK was circular, but it is not.
-- Keeping it because it is a normal 1:1/1:N relationship from tokens -> users.
ALTER TABLE valid.tokens
    ADD CONSTRAINT user_id_tokens_foreign FOREIGN KEY (user_id) REFERENCES valid.users (user_id);
