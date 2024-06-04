CREATE SCHEMA IF NOT EXISTS valid;
CREATE TABLE IF NOT EXISTS valid.salt(
    id INT8 GENERATED ALWAYS AS IDENTITY,
    salt UUID NOT NULL
);
ALTER TABLE
    valid.salt ADD PRIMARY KEY(id);
ALTER TABLE 
valid.salt ADD CONSTRAINT IF NOT EXISTS salt_unique_salt UNIQUE(salt);

CREATE TABLE IF NOT EXISTS valid.username(
    id INT8 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    username STRING(255) NOT NULL
);
ALTER TABLE
    valid.username ADD CONSTRAINT username_username_unique UNIQUE(username);
CREATE TABLE IF NOT EXISTS valid.password(
    id INT8 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    password STRING(255) NOT NULL
);
CREATE TABLE IF NOT EXISTS valid.email(
    id INT8 PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    email STRING(255) NOT NULL
);
ALTER TABLE
    valid.email ADD CONSTRAINT email_email_unique UNIQUE(email);
CREATE TABLE IF NOT EXISTS valid.users(
    id UUID PRIMARY KEY NOT NULL,
    email INT8 NOT NULL,
    username INT8 NOT NULL,
    password INT8 NOT NULL,
    salt INT8 NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    firstname STRING(255) NOT NULL,
    lastname STRING(255) NOT NULL
);
CREATE INDEX IF NOT EXISTS users_username_index ON
    valid.users(username);
CREATE INDEX IF NOT EXISTS users_firstname_index ON
valid.users(firstname);
CREATE INDEX IF NOT EXISTS users_lastname_index ON
    valid.users(lastname);
CREATE TABLE IF NOT EXISTS valid.ip(
    id INT8 PRIMARY KEY NOT NULL,
    ip STRING(255) NOT NULL,
    userId UUID NOT NULL
);
ALTER TABLE
    valid.users ADD CONSTRAINT users_salt_foreign FOREIGN KEY(salt) REFERENCES valid.salt(id);
ALTER TABLE
    valid.users ADD CONSTRAINT users_username_foreign FOREIGN KEY(username) REFERENCES valid.username(id);
ALTER TABLE
    valid.ip ADD CONSTRAINT ips_userid_foreign FOREIGN KEY(userId) REFERENCES valid.users(id);
ALTER TABLE
    valid.users ADD CONSTRAINT users_password_foreign FOREIGN KEY(password) REFERENCES valid.password(id);
ALTER TABLE
    valid.users ADD CONSTRAINT users_email_foreign FOREIGN KEY(email) REFERENCES valid.email(id);
