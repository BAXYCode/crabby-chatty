-- Add down migration script here

DROP TABLE IF EXISTS valid.users CASCADE; 
DROP TABLE IF EXISTS valid.email CASCADE; 
DROP TABLE IF EXISTS valid.password CASCADE; 
DROP TABLE IF EXISTS valid.username CASCADE; 
DROP TABLE IF EXISTS valid.ip CASCADE; 
