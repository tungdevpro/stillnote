-- Stillnote schema. Run once in Supabase: Dashboard → SQL Editor → paste → Run.
-- (Or with the Supabase CLI: `supabase db push`.)
--
-- Timestamps are Unix milliseconds (bigint) set by the app.
-- `server_seq` is assigned by the server on every accepted write and is what
-- clients use to pull changes since their last sync.

create table if not exists public.notebooks (
    id          uuid primary key,
    user_id     uuid not null default auth.uid() references auth.users (id) on delete cascade,
    name        text not null,
    created_at  bigint not null,
    updated_at  bigint not null,
    deleted     boolean not null default false,
    server_seq  bigint not null default 0
);

create table if not exists public.notes (
    id          uuid primary key,
    user_id     uuid not null default auth.uid() references auth.users (id) on delete cascade,
    notebook_id uuid,
    title       text not null default '',
    body        text not null default '',
    tags        text[] not null default '{}',
    pinned      boolean not null default false,
    trashed_at  bigint,
    created_at  bigint not null,
    updated_at  bigint not null,
    deleted     boolean not null default false,
    server_seq  bigint not null default 0
);

create index if not exists notebooks_user_seq on public.notebooks (user_id, server_seq);
create index if not exists notes_user_seq on public.notes (user_id, server_seq);

-- Last write wins: an update carrying an older `updated_at` than the stored
-- row is silently skipped. Accepted writes get a fresh `server_seq`.
create or replace function public.stillnote_before_write()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
    if tg_op = 'UPDATE' then
        if new.updated_at < old.updated_at then
            return null;
        end if;
        new.user_id := old.user_id;
    end if;
    new.server_seq := (extract(epoch from clock_timestamp()) * 1000)::bigint;
    return new;
end;
$$;

drop trigger if exists notebooks_before_write on public.notebooks;
create trigger notebooks_before_write
    before insert or update on public.notebooks
    for each row execute function public.stillnote_before_write();

drop trigger if exists notes_before_write on public.notes;
create trigger notes_before_write
    before insert or update on public.notes
    for each row execute function public.stillnote_before_write();

-- Row Level Security: every user only sees and writes their own rows.
alter table public.notebooks enable row level security;
alter table public.notes enable row level security;

drop policy if exists "own notebooks" on public.notebooks;
create policy "own notebooks" on public.notebooks
    for all to authenticated
    using ((select auth.uid()) = user_id)
    with check ((select auth.uid()) = user_id);

drop policy if exists "own notes" on public.notes;
create policy "own notes" on public.notes
    for all to authenticated
    using ((select auth.uid()) = user_id)
    with check ((select auth.uid()) = user_id);
