-- Add up migration script here

create schema if not exists validation;

create extension if not exists citext;
create extension if not exists pgcrypto;

create table if not exists validation.auth_user (
    user_id            uuid primary key default gen_random_uuid(),

    email              citext not null,
    username           citext not null,

    password_hash      text not null,

    -- is_email_verified  boolean not null default false,
    -- email_verified_at  timestamptz null,

    created_at         timestamptz not null default now(),
    updated_at         timestamptz not null default now(),

    constraint uq_auth_user_email unique (email),
    constraint uq_auth_user_username unique (username)

    -- constraint chk_email_verified_at_consistency
    --     check ((is_email_verified = false and email_verified_at is null) or (is_email_verified = true))
);

