-- Add up migration script here
create table if not exists validation.paseto_public_key (
    key_id         uuid primary key default gen_random_uuid(),

    -- key identifier stored in the PASETO footer
    kid            text not null,

    -- PASERK-encoded public key (e.g. "k4.public....")
    public_paserk  bytea not null,

    created_at     timestamptz not null default now(),

    constraint uq_paseto_public_kid unique (kid),
    constraint uq_paseto_public_paserk unique (public_paserk)
);

create index if not exists ix_paseto_public_kid on validation.paseto_public_key (kid);

