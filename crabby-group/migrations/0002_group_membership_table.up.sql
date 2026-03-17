-- Add up migration script here
CREATE TYPE role AS ENUM ('admin', 'member');
CREATE table group_membership(
    group_id            UUID NOT NULL,
    user_id             UUID NOT NULL,
    role                role, 
    joined_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, user_id),
    FOREIGN KEY (group_id) REFERENCES chat_group(group_id)ON DELETE CASCADE
);

CREATE TABLE group_membership_version(
    group_id            UUID PRIMARY KEY,
    version     BIGINT NOT NULL,
    FOREIGN KEY (group_id) REFERENCES chat_group(group_id) ON DELETE CASCADE
);

CREATE OR REPLACE FUNCTION bump_group_membership_version()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
DECLARE
    affected_group_id UUID;
BEGIN
    affected_group_id := COALESCE(NEW.group_id, OLD.group_id);

    INSERT INTO group_membership_version (group_id, version)
    VALUES (affected_group_id, 1)
    ON CONFLICT (group_id)
    DO UPDATE
    SET version = group_membership_version.version + 1;

    RETURN COALESCE(NEW, OLD);
END;
$$;



CREATE OR REPLACE TRIGGER trigger_bump_group_membership_version
AFTER INSERT OR UPDATE OR DELETE
ON group_membership
FOR EACH ROW
EXECUTE FUNCTION bump_group_membership_version();

