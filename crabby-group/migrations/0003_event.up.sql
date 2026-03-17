-- Add up migration script here
CREATE TYPE event_type AS ENUM ('added', 'removed','left');

CREATE TABLE group_event(
    event_id    UUID NOT NULL DEFAULT uuidv7(),
    subject_id  UUID NOT NULL,
    actor_id    UUID NOT NULL,
    group_id    UUID NOT NULL,
    event_type  event_type NOT NULL,
    occured_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (event_id),
    FOREIGN KEY (group_id) REFERENCES chat_group(group_id)
);
