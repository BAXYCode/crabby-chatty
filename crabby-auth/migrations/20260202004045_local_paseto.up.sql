-- Add up migration script here
create table if not exists validation.paseto_local_wrap_key (
    key_id             uuid primary key default gen_random_uuid(),

    -- key identifier stored in the PASETO footer
    kid                text not null,

    -- PASERK-encoded wrapped local key (e.g. "k4.local-wrap.pie....")
    -- TODO: actually wrap this key with super secret key
    local_wrap_paserk  bytea not null,

    created_at         timestamptz not null default now(),

    constraint uq_paseto_local_wrap_kid unique (kid),
    constraint uq_paseto_local_wrap_paserk unique (local_wrap_paserk)
);

create index if not exists ix_paseto_local_wrap_kid on validation.paseto_local_wrap_key (kid);

