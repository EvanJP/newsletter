# Newsletter API

## To-Do
- [ ] Make CI/CD integration with fly. https://fly.io/docs/reference/configuration/

## Commands
* `./scripts/init_db.sh`
    - Starts up the Postgres docker container.
    - `docker stop <container_id>`

## Env
- If you want all logs for test cases:
    - `TEST_LOG=true cargo test health_check_works`
    - Can be piped to `bunyan`. 