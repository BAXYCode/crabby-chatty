-- Add up migration script here
CREATE USER IF NOT EXISTS auth_login;

GRANT ALL ON DATABASE crabby_authdb TO auth_login;

ALTER DATABASE crabby_authdb OWNER TO auth_login;

-- ALTER USER auth_login WITH PASSWORD 'authdb';
