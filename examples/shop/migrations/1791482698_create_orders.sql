CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    product TEXT NOT NULL,
    quantity BIGINT NOT NULL,
    total BIGINT NOT NULL,
    status TEXT NOT NULL
);
