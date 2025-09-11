CREATE SCHEMA IF NOT EXISTS valid;

CREATE TABLE IF NOT EXISTS valid.username(
    id INT8 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    username STRING(255) NOT NULL
);
ALTER TABLE
    valid.username ADD CONSTRAINT IF NOT EXISTS username_username_unique UNIQUE(username);
CREATE TABLE IF NOT EXISTS valid.password(
    id INT8 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    password STRING(255) NOT NULL
);
CREATE TABLE IF NOT EXISTS valid.email(
    id INT8 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    email STRING(255) NOT NULL
);
ALTER TABLE
    valid.email ADD CONSTRAINT IF NOT EXISTS email_email_unique UNIQUE(email);

CREATE TABLE IF NOT EXISTS valid.last_login(
    id SERIAL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    last_login TIMESTAMPTZ NOT NULL DEFAULT current_timestamp()
);
CREATE TABLE IF NOT EXISTS valid.users(
    user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email_id INT8 NOT NULL,
    username_id INT8 NOT NULL,
    password_id INT8 NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp(),
    last_login_id SERIAL NOT NULL,
    is_admin BOOL NOT NULL);

CREATE INDEX IF NOT EXISTS users_username_index ON
    valid.username(username);
CREATE INDEX IF NOT EXISTS user_id_index ON
    valid.users(user_id);
CREATE TABLE IF NOT EXISTS valid.ip(
    id INT8 PRIMARY KEY NOT NULL,
    ip STRING(255) NOT NULL,
    userId UUID NOT NULL
);
ALTER TABLE
    valid.users ADD CONSTRAINT users_username_foreign FOREIGN KEY(username_id) REFERENCES valid.username(id);
ALTER TABLE
    valid.ip ADD CONSTRAINT ips_userid_foreign FOREIGN KEY(userId) REFERENCES valid.users(user_id);
ALTER TABLE
    valid.users ADD CONSTRAINT users_password_foreign FOREIGN KEY(password_id) REFERENCES valid.password(id);
ALTER TABLE
    valid.users ADD CONSTRAINT users_email_foreign FOREIGN KEY(email_id) REFERENCES valid.email(id);
ALTER TABLE
    valid.users ADD CONSTRAINT users_last_login_foreign FOREIGN KEY(last_login_id) REFERENCES valid.last_login(id);
