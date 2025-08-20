-- Add up migration script here
CREATE USER IF NOT EXISTS user_login;

GRANT ALL ON DATABASE crabby_userdb TO user_login;

ALTER DATABASE crabby_userdb OWNER TO user_login;

-- ALTER USER user_login WITH PASSWORD "userdb";
