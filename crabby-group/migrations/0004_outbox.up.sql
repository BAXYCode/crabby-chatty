-- Add up migration script here

CREATE TABLE outbox(
    group_id    UUID NOT NULL,
    event_id UUID NOT NULL, 
    event_type event_type NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at TIMESTAMPTZ,
    PRIMARY KEY (event_id),
    FOREIGN KEY (group_id) REFERENCES chat_group(group_id),
    FOREIGN KEY (event_id) REFERENCES group_event(event_id)
);


CREATE OR REPLACE FUNCTION write_group_event_to_outbox()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO outbox (
        group_id,
        event_id,
        event_type
    )
    VALUES (
        NEW.group_id,
        NEW.event_id,
        NEW.event_type
    )
    ON CONFLICT DO NOTHING;

    RETURN NEW;
END;
$$;


CREATE OR REPLACE TRIGGER trigger_write_group_event_to_outbox
AFTER INSERT
ON group_event
FOR EACH ROW
EXECUTE FUNCTION write_group_event_to_outbox();
