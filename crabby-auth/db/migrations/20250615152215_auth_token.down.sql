-- Add down migration script here

DROP TABLE IF EXISTS valid.tokens CASCADE;
DROP TABLE IF EXISTS valid.bearer CASCADE;
DROP TABLE IF EXISTS valid.refresh CASCADE;
