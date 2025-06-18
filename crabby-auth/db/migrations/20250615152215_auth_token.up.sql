-- Add up migration script here

CREATE TABLE IF NOT EXISTS valid.tokens(
    user_id UUID NOT NULL PRIMARY KEY,
    bearer_id SERIAL NOT NULL,
   refresh_id SERIAL NOT NULL);
CREATE TABLE IF NOT EXISTS valid.bearer(
    id SERIAL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    bearer STRING NOT NULL);
CREATE TABLE IF NOT EXISTS valid.refresh(
    id SERIAL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    refresh STRING NOT NULL);
ALTER TABLE
    valid.tokens ADD CONSTRAINT token_bearer_foreign FOREIGN KEY(bearer_id) REFERENCES valid.bearer(id);
ALTER TABLE
    valid.tokens ADD CONSTRAINT token_refresh_foreign FOREIGN KEY(refresh_id) REFERENCES valid.refresh(id);
ALTER TABLE
    valid.tokens ADD CONSTRAINT user_id_tokens_foreign FOREIGN KEY(user_id) REFERENCES valid.users(user_id);
