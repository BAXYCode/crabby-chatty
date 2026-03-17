-- Add up migration script here

CREATE TABLE chat_group (
    group_id            UUID NOT NULL DEFAULT uuidv7(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id)
);
