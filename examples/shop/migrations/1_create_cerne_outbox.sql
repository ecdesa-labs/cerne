-- The commands fired by policies, stored in the same transaction as the aggregate that produced the events.
CREATE TABLE cerne_outbox (
    id INTEGER PRIMARY KEY,
    command TEXT NOT NULL,
    json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    error TEXT
);
