-- A mint's URL can change; its NUT-06 public key cannot. Give the mint row a
-- surrogate id so a move is a single-column update, and resolve every caller
-- identifier (public key or URL) through mint_alias. Every alias except a
-- superseded URL is derivable from the mint rows, so a damaged index can be
-- rebuilt at the cost of old URLs no longer resolving.

-- mint_url is a contact address, not an identity: mint_alias decides which row
-- a URL resolves to, so no uniqueness is declared here.
CREATE TABLE mint_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mint_url TEXT NOT NULL,
    name TEXT,
    pubkey BLOB,
    version TEXT,
    description TEXT,
    description_long TEXT,
    contact TEXT,
    nuts TEXT,
    motd TEXT,
    icon_url TEXT,
    mint_time INTEGER,
    urls TEXT,
    tos_url TEXT
);

INSERT INTO mint_new (
    mint_url, name, pubkey, version, description, description_long,
    contact, nuts, motd, icon_url, mint_time, urls, tos_url
)
SELECT
    mint_url, name, pubkey, version, description, description_long,
    contact, nuts, motd, icon_url, mint_time, urls, tos_url
FROM mint
ORDER BY mint_url;

-- The previous update_mint_url moved proofs and mint quotes to a URL without
-- moving the mint row, so a wallet that ran it has rows pointing at a URL with
-- no mint. Recreate those mints rather than dropping the rows below.
INSERT INTO mint_new (mint_url)
SELECT s.mint_url FROM (
SELECT mint_url FROM proof
UNION
SELECT mint_url FROM keyset
UNION
SELECT mint_url FROM mint_quote
UNION
SELECT mint_url FROM transactions
UNION
SELECT mint_url FROM wallet_sagas
UNION
SELECT mint_url FROM melt_quote WHERE mint_url IS NOT NULL
) s
WHERE NOT EXISTS (SELECT 1 FROM mint_new m WHERE m.mint_url = s.mint_url);

DROP TABLE mint;
ALTER TABLE mint_new RENAME TO mint;

-- Every identifier a caller may hold resolves here. Old URLs are kept after a
-- move so stale references still resolve. Duplicate public keys (the same mint
-- added twice under different URLs) leave the alias with the lowest mint id;
-- the other row stays reachable by URL.
CREATE TABLE mint_alias (
    alias TEXT PRIMARY KEY,
    mint_id INTEGER NOT NULL REFERENCES mint(id) ON DELETE CASCADE
);
CREATE INDEX mint_alias_mint_id_index ON mint_alias(mint_id);

INSERT OR IGNORE INTO mint_alias (alias, mint_id)
SELECT mint_url, id FROM mint ORDER BY id;

INSERT OR IGNORE INTO mint_alias (alias, mint_id)
SELECT lower(hex(pubkey)), id FROM mint WHERE pubkey IS NOT NULL ORDER BY id;

CREATE TABLE keyset_new (
    id TEXT PRIMARY KEY,
    mint_id INTEGER NOT NULL REFERENCES mint(id) ON DELETE CASCADE,
    keyset_u32 INTEGER,
    unit TEXT NOT NULL,
    active BOOL NOT NULL,
    input_fee_ppk INTEGER,
    final_expiry INTEGER DEFAULT NULL
);
INSERT INTO keyset_new (id, mint_id, keyset_u32, unit, active, input_fee_ppk, final_expiry)
SELECT k.id, m.id, k.keyset_u32, k.unit, k.active, k.input_fee_ppk, k.final_expiry
FROM keyset k JOIN mint m ON m.mint_url = k.mint_url;
DROP TABLE keyset;
ALTER TABLE keyset_new RENAME TO keyset;
CREATE INDEX keyset_mint_id_index ON keyset(mint_id);

CREATE TABLE proof_new (
    y BLOB PRIMARY KEY,
    mint_id INTEGER NOT NULL REFERENCES mint(id) ON DELETE CASCADE,
    state TEXT CHECK ( state IN ('SPENT', 'UNSPENT', 'PENDING', 'RESERVED', 'PENDING_SPENT' ) ) NOT NULL,
    spending_condition TEXT,
    unit TEXT NOT NULL,
    amount INTEGER NOT NULL,
    keyset_id TEXT NOT NULL,
    secret TEXT NOT NULL,
    c BLOB NOT NULL,
    witness TEXT,
    dleq_e BLOB,
    dleq_s BLOB,
    dleq_r BLOB,
    p2pk_e BLOB,
    used_by_operation TEXT,
    created_by_operation TEXT
);
INSERT INTO proof_new (
    y, mint_id, state, spending_condition, unit, amount, keyset_id, secret, c,
    witness, dleq_e, dleq_s, dleq_r, p2pk_e, used_by_operation, created_by_operation
)
SELECT
    p.y, m.id, p.state, p.spending_condition, p.unit, p.amount, p.keyset_id,
    p.secret, p.c, p.witness, p.dleq_e, p.dleq_s, p.dleq_r, p.p2pk_e,
    p.used_by_operation, p.created_by_operation
FROM proof p JOIN mint m ON m.mint_url = p.mint_url;
DROP TABLE proof;
ALTER TABLE proof_new RENAME TO proof;
CREATE INDEX proof_mint_id_index ON proof(mint_id);
CREATE INDEX proof_used_by_operation_index ON proof(used_by_operation);
CREATE INDEX proof_created_by_operation_index ON proof(created_by_operation);

CREATE TABLE mint_quote_new (
    id TEXT PRIMARY KEY,
    mint_id INTEGER NOT NULL REFERENCES mint(id) ON DELETE CASCADE,
    payment_method TEXT NOT NULL DEFAULT 'bolt11',
    amount INTEGER,
    unit TEXT NOT NULL,
    request TEXT NOT NULL,
    state TEXT NOT NULL,
    expiry INTEGER NOT NULL,
    amount_paid INTEGER NOT NULL DEFAULT 0,
    amount_issued INTEGER NOT NULL DEFAULT 0,
    secret_key TEXT,
    created_time INTEGER NOT NULL DEFAULT 0,
    used_by_operation TEXT,
    version INTEGER NOT NULL DEFAULT 0,
    estimated_blocks INTEGER,
    updated_at INTEGER NOT NULL DEFAULT 0
);
INSERT INTO mint_quote_new (
    id, mint_id, payment_method, amount, unit, request, state, expiry,
    amount_paid, amount_issued, secret_key, created_time, used_by_operation,
    version, estimated_blocks, updated_at
)
SELECT
    q.id, m.id, q.payment_method, q.amount, q.unit, q.request, q.state, q.expiry,
    q.amount_paid, q.amount_issued, q.secret_key, q.created_time, q.used_by_operation,
    q.version, q.estimated_blocks, q.updated_at
FROM mint_quote q JOIN mint m ON m.mint_url = q.mint_url;
DROP TABLE mint_quote;
ALTER TABLE mint_quote_new RENAME TO mint_quote;
CREATE INDEX mint_quote_mint_id_index ON mint_quote(mint_id);
CREATE INDEX idx_mint_quote_pending ON mint_quote(payment_method, amount_issued);
CREATE INDEX mint_quote_used_by_operation_index ON mint_quote(used_by_operation);

-- melt_quote.mint_url was added late and is nullable, so mint_id stays nullable
-- and rows that never recorded a mint keep NULL instead of being dropped.
CREATE TABLE melt_quote_new (
    id TEXT PRIMARY KEY,
    unit TEXT NOT NULL,
    amount INTEGER NOT NULL,
    request TEXT NOT NULL,
    fee_reserve INTEGER NOT NULL,
    expiry INTEGER NOT NULL,
    state TEXT CHECK ( state IN ('UNPAID', 'PENDING', 'PAID' ) ) NOT NULL DEFAULT 'UNPAID',
    payment_proof TEXT,
    payment_method TEXT NOT NULL DEFAULT 'bolt11',
    used_by_operation TEXT,
    version INTEGER NOT NULL DEFAULT 0,
    mint_id INTEGER REFERENCES mint(id) ON DELETE CASCADE,
    estimated_blocks INTEGER,
    fee_index INTEGER
);
INSERT INTO melt_quote_new (
    id, unit, amount, request, fee_reserve, expiry, state, payment_proof,
    payment_method, used_by_operation, version, mint_id, estimated_blocks, fee_index
)
SELECT
    q.id, q.unit, q.amount, q.request, q.fee_reserve, q.expiry, q.state, q.payment_proof,
    q.payment_method, q.used_by_operation, q.version, m.id, q.estimated_blocks, q.fee_index
FROM melt_quote q LEFT JOIN mint m ON m.mint_url = q.mint_url;
DROP TABLE melt_quote;
ALTER TABLE melt_quote_new RENAME TO melt_quote;
CREATE INDEX melt_quote_mint_id_index ON melt_quote(mint_id);
CREATE INDEX melt_quote_state_index ON melt_quote(state);
CREATE INDEX melt_quote_used_by_operation_index ON melt_quote(used_by_operation);

CREATE TABLE transactions_new (
    id BLOB PRIMARY KEY,
    mint_id INTEGER NOT NULL REFERENCES mint(id) ON DELETE CASCADE,
    direction TEXT CHECK (direction IN ('Incoming', 'Outgoing')) NOT NULL,
    amount INTEGER NOT NULL,
    fee INTEGER NOT NULL,
    unit TEXT NOT NULL,
    ys BLOB NOT NULL,
    timestamp INTEGER NOT NULL,
    memo TEXT,
    metadata TEXT,
    quote_id TEXT,
    payment_request TEXT,
    payment_proof TEXT,
    payment_method TEXT,
    saga_id TEXT,
    status TEXT NOT NULL DEFAULT 'completed' CHECK (status IN ('pending', 'completed', 'failed'))
);
INSERT INTO transactions_new (
    id, mint_id, direction, amount, fee, unit, ys, timestamp, memo, metadata,
    quote_id, payment_request, payment_proof, payment_method, saga_id, status
)
SELECT
    t.id, m.id, t.direction, t.amount, t.fee, t.unit, t.ys, t.timestamp, t.memo, t.metadata,
    t.quote_id, t.payment_request, t.payment_proof, t.payment_method, t.saga_id, t.status
FROM transactions t JOIN mint m ON m.mint_url = t.mint_url;
DROP TABLE transactions;
ALTER TABLE transactions_new RENAME TO transactions;
CREATE INDEX transactions_mint_id_index ON transactions(mint_id);
CREATE INDEX direction_index ON transactions(direction);
CREATE INDEX unit_index ON transactions(unit);
CREATE INDEX timestamp_index ON transactions(timestamp);
CREATE INDEX transactions_saga_id_index ON transactions(saga_id);

CREATE TABLE wallet_sagas_new (
    id TEXT PRIMARY KEY,
    kind TEXT CHECK (kind IN ('send', 'receive', 'swap', 'mint', 'melt')) NOT NULL,
    state TEXT NOT NULL,
    amount INTEGER NOT NULL,
    mint_id INTEGER NOT NULL REFERENCES mint(id) ON DELETE CASCADE,
    unit TEXT NOT NULL,
    quote_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    data TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0
);
INSERT INTO wallet_sagas_new (
    id, kind, state, amount, mint_id, unit, quote_id, created_at, updated_at, data, version
)
SELECT
    s.id, s.kind, s.state, s.amount, m.id, s.unit, s.quote_id, s.created_at, s.updated_at,
    s.data, s.version
FROM wallet_sagas s JOIN mint m ON m.mint_url = s.mint_url;
DROP TABLE wallet_sagas;
ALTER TABLE wallet_sagas_new RENAME TO wallet_sagas;
CREATE INDEX wallet_sagas_mint_id_index ON wallet_sagas(mint_id);
CREATE INDEX wallet_sagas_kind_index ON wallet_sagas(kind);
CREATE INDEX wallet_sagas_created_at_index ON wallet_sagas(created_at);
