-- Identify a mint by the pubkey it publishes rather than by its URL.
--
-- Nothing writes to these yet. A mint moves onto them the first time the wallet
-- talks to it and learns a pubkey; mints that publish none stay in `mint`.

-- The mint itself. Keys are lowercase hex rather than a blob so that every
-- backend, including the REST-based one that cannot store blobs, uses the same
-- representation.
CREATE TABLE IF NOT EXISTS mint_identity (
    pubkey            TEXT PRIMARY KEY,
    name              TEXT,
    version           TEXT,
    description       TEXT,
    description_long  TEXT,
    contact           TEXT,
    nuts              TEXT,
    motd              TEXT,
    icon_url          TEXT,
    mint_time         INTEGER,
    urls              TEXT,
    tos_url           TEXT,
    first_seen        INTEGER NOT NULL,
    last_seen         INTEGER NOT NULL
);

-- Where an identity can be reached. Mints are still added by URL and tokens are
-- still URL-addressed, so this is how either resolves to an identity.
CREATE TABLE IF NOT EXISTS mint_locator (
    mint_url           TEXT PRIMARY KEY,
    pubkey             TEXT NOT NULL,
    source             TEXT NOT NULL CHECK (source IN ('contacted', 'corroborated', 'accepted')),
    added_time         INTEGER NOT NULL,
    last_verified_time INTEGER NOT NULL,
    FOREIGN KEY (pubkey) REFERENCES mint_identity(pubkey) ON UPDATE CASCADE ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS mint_locator_pubkey_index ON mint_locator(pubkey);

-- Associations the wallet saw but refused to apply. The pubkey is not
-- authenticated, so pooling two URLs' funds under one identity needs a person.
CREATE TABLE IF NOT EXISTS mint_identity_claim (
    mint_url   TEXT NOT NULL,
    pubkey     TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('merge', 'rotation')),
    status     TEXT NOT NULL CHECK (status IN ('pending', 'rejected')),
    first_seen INTEGER NOT NULL,
    last_seen  INTEGER NOT NULL,
    PRIMARY KEY (mint_url, pubkey)
);

-- Rows carry the identity they belong to. NULL means the mint has not moved off
-- the URL-keyed table yet, in which case mint_url is still the identity.
ALTER TABLE keyset ADD mint_pubkey TEXT;
ALTER TABLE mint_quote ADD mint_pubkey TEXT;
ALTER TABLE melt_quote ADD mint_pubkey TEXT;
ALTER TABLE proof ADD mint_pubkey TEXT;
ALTER TABLE transactions ADD mint_pubkey TEXT;
ALTER TABLE wallet_sagas ADD mint_pubkey TEXT;

CREATE INDEX IF NOT EXISTS keyset_mint_pubkey_index ON keyset(mint_pubkey);
CREATE INDEX IF NOT EXISTS mint_quote_mint_pubkey_index ON mint_quote(mint_pubkey);
CREATE INDEX IF NOT EXISTS melt_quote_mint_pubkey_index ON melt_quote(mint_pubkey);
CREATE INDEX IF NOT EXISTS proof_mint_pubkey_index ON proof(mint_pubkey);
CREATE INDEX IF NOT EXISTS transactions_mint_pubkey_index ON transactions(mint_pubkey);
CREATE INDEX IF NOT EXISTS wallet_sagas_mint_pubkey_index ON wallet_sagas(mint_pubkey);
