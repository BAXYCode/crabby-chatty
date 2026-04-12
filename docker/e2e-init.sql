-- Combined group service migrations for e2e testing
-- Source: crabby-group/migrations/0001-0004

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Helper for UUIDv7 if not available
CREATE OR REPLACE FUNCTION uuidv7() RETURNS uuid
LANGUAGE sql VOLATILE AS $$
  SELECT encode(
    set_bit(
      set_bit(
        overlay(
          uuid_send(gen_random_uuid())
          placing substring(int8send((extract(epoch from clock_timestamp()) * 1000)::bigint) from 3)
          from 1 for 6
        ),
        52, 1
      ),
      53, 1
    ),
    'hex'
  )::uuid;
$$;

-- 0001: chat_group
CREATE TABLE chat_group (
    group_id    UUID NOT NULL DEFAULT uuidv7(),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id)
);

-- 0002: group_membership + version + trigger
CREATE TYPE role AS ENUM ('admin', 'member');

CREATE TABLE group_membership (
    group_id  UUID NOT NULL,
    user_id   UUID NOT NULL,
    role      role,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, user_id),
    FOREIGN KEY (group_id) REFERENCES chat_group(group_id) ON DELETE CASCADE
);

CREATE TABLE group_membership_version (
    group_id UUID PRIMARY KEY,
    version  BIGINT NOT NULL,
    FOREIGN KEY (group_id) REFERENCES chat_group(group_id) ON DELETE CASCADE
);

CREATE OR REPLACE FUNCTION bump_group_membership_version()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    affected_group_id UUID;
BEGIN
    affected_group_id := COALESCE(NEW.group_id, OLD.group_id);
    INSERT INTO group_membership_version (group_id, version)
    VALUES (affected_group_id, 1)
    ON CONFLICT (group_id)
    DO UPDATE SET version = group_membership_version.version + 1;
    RETURN COALESCE(NEW, OLD);
END;
$$;

CREATE OR REPLACE TRIGGER trigger_bump_group_membership_version
AFTER INSERT OR UPDATE OR DELETE ON group_membership
FOR EACH ROW EXECUTE FUNCTION bump_group_membership_version();

-- 0003: group_event
CREATE TYPE event_type AS ENUM ('added', 'removed', 'left');

CREATE TABLE group_event (
    event_id   UUID NOT NULL DEFAULT uuidv7(),
    subject_id UUID NOT NULL,
    actor_id   UUID NOT NULL,
    group_id   UUID NOT NULL,
    event_type event_type NOT NULL,
    occured_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (event_id),
    FOREIGN KEY (group_id) REFERENCES chat_group(group_id)
);

-- 0004: outbox
CREATE TABLE outbox (
    group_id     UUID NOT NULL,
    event_id     UUID NOT NULL,
    event_type   event_type NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at TIMESTAMPTZ,
    PRIMARY KEY (event_id),
    FOREIGN KEY (group_id) REFERENCES chat_group(group_id),
    FOREIGN KEY (event_id) REFERENCES group_event(event_id)
);

CREATE OR REPLACE FUNCTION write_group_event_to_outbox()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO outbox (group_id, event_id, event_type)
    VALUES (NEW.group_id, NEW.event_id, NEW.event_type)
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE TRIGGER trigger_write_group_event_to_outbox
AFTER INSERT ON group_event
FOR EACH ROW EXECUTE FUNCTION write_group_event_to_outbox();
