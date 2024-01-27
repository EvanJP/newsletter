# Newsletter API

![Main CI Workflow](https://github.com/EvanJP/newsletter/actions/workflows/main.yml/badge.svg)
![Audit CI Workflow](https://github.com/EvanJP/newsletter/actions/workflows/audit.yml/badge.svg)

A simple newsletter API built on [Actix](https://actix.rs/), using the
[Sendgrid API](https://sendgrid.com/en-us).

## Features

- Completely built with Rust w/ Postgres + Docker + Fly.io support.
- Full unit/integration testing.
- API for registering/sending/monitoring newsletter subscriptions.
- Argon2 based authentication.
- Full Tracing Logging with [`tracing`](https://docs.rs/tracing/latest/tracing/).
- HMAC secrecy for query params.
- (Almost) Fully Documented.

## API

- `/`
  - To the home page.
- `/login`
  - To the login form.
- `/health_check`
  - A HTTP 200 health check.
- `/subscriptions`
  - A `POST` to subscribe to the newsletter.
- `/subscriptions/confirm`
  - Confirming subscriptions.
- `/newsletters`
  - A `POST` to send out newsletters to anyone who is a confirmed subscriber.

## Commands

- `cargo test`
  - Runs all the integration testing and unit tests.
- `cargo doc --open`
  - Generate the documentation and opens it.
- `./scripts/init_db.sh`
  - Starts up the local Postgres docker container.
  - `docker stop <container_id>`
- `cargo sqlx prepare -- --lib`
  - Generates Sqlx Offline Mode files.
  - Sqlx calls our database at compile time to ensure queries can be ran
    successfully, but our Docker image can't connect to the `DATABASE_URL`.
    Offline files ensure that we can build the docker image.

## Env

- If you want all logs for test cases:
  - `TEST_LOG=true cargo test health_check_works`
  - Can be piped to `bunyan`.

## Column Migrations

### Mandatory Columns

1. Generate new migration script with `sqlx migrate add [name]`.
2. `ALTER TABLE` with null/optional availability.
   - `SKIP_DOCKER=true ./scripts/init_db.sh` To test locally.
3. Proxy into the postgres container and migrate with `sqlx migrate run`.
4. Deploy with `fly deploy`.
5. `sqlx migrate add [name]` and backfill the table. E.g.

```sql
BEGIN;
    UPDATE subscriptions
        SET status = 'confirmed'
        WHERE status IS NULL;
    ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;
```

## To run SQLX on fly:

- Proxy with `fly proxy 15432:5432 -a <pg name>` then psql into it.
