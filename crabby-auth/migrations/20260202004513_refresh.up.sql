-- Add up migration script here
create table if not exists validation.refresh_token (
    refresh_token_id  uuid primary key default gen_random_uuid(),

    user_id           uuid not null
        references validation.auth_user(user_id) on delete cascade,

    -- unique token instance id stored in the token claims
    token_jti         uuid not null,

    -- hash of the *raw refresh token* (e.g. sha-256 digest bytes)
    token_hash        bytea not null,


    issued_at         timestamptz not null default now(),
    expires_at        timestamptz not null,

    constraint uq_refresh_token_user_jti unique (user_id, token_jti),
    constraint uq_refresh_token_hash unique (token_hash),
    constraint chk_refresh_token_expiry check (expires_at > issued_at)
);

create index if not exists ix_refresh_token_user_expiry
    on validation.refresh_token (user_id, expires_at);


