-- Add down migration script here
REVOKE ALL ON DATABASE userdb FROM user_login;

ALTER DATABASE crabby_userdb OWNER TO root;

DROP USER user_login;
