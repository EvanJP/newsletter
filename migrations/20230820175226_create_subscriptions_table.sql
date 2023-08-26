CREATE TABLE subscriptions(
    id uuid NOT NULL,
    PRIMARY KEY (id),
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    -- timestamptz is timezone aware.
    subscribed_at timestamptz NOT NULL
)