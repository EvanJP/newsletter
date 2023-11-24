# Newsletter API

![Main CI Workflow](https://github.com/EvanJP/newsletter/actions/workflows/main.yml/badge.svg)
![Audit CI Workflow](https://github.com/EvanJP/newsletter/actions/workflows/audit.yml/badge.svg)

## To-Do

- [ ] Make CI/CD integration with fly. https://fly.io/docs/reference/configuration/

## Commands

- `./scripts/init_db.sh`
  - Starts up the Postgres docker container.
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
