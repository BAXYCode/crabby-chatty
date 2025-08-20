-- Add down migration script here
REVOKE ALL ON DATABASE authdb FROM  auth_login;

ALTER DATABASE crabby_authdb OWNER TO root;

DROP USER auth_login;
