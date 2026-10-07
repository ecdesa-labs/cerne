-- The commands fired by policies, stored in the same transaction as the aggregate (D32).
CREATE TABLE cerne_outbox (
    id INTEGER PRIMARY KEY,
    command TEXT NOT NULL,
    json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    error TEXT
);
