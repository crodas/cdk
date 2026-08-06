-- Drop the keyset -> mint foreign key so a mint can be identified by its pubkey.
--
-- A mint promoted to the pubkey-keyed store leaves the URL-keyed `mint` table,
-- which the foreign key would reject. Keyset deletion is now explicit in
-- `remove_mint` instead of relying on ON DELETE CASCADE.
CREATE TABLE keyset_new (
    id TEXT PRIMARY KEY,
    mint_url TEXT NOT NULL,
    keyset_u32 INTEGER,
    unit TEXT NOT NULL,
    active BOOL NOT NULL,
    input_fee_ppk INTEGER,
    final_expiry INTEGER DEFAULT NULL
);

INSERT INTO keyset_new (id, mint_url, keyset_u32, unit, active, input_fee_ppk, final_expiry)
SELECT id, mint_url, keyset_u32, unit, active, input_fee_ppk, final_expiry
FROM keyset;

DROP TABLE keyset;

ALTER TABLE keyset_new RENAME TO keyset;

-- Rewriting a mint's identity filters every dependent table by mint_url, so each
-- needs its own index. Names are qualified because SQLite index names are global
-- to the database: the unqualified `mint_url_index` was already taken by `proof`,
-- which silently turned the `transactions` one into a no-op.
CREATE INDEX IF NOT EXISTS keyset_mint_url_index ON keyset(mint_url);
CREATE INDEX IF NOT EXISTS mint_quote_mint_url_index ON mint_quote(mint_url);
CREATE INDEX IF NOT EXISTS melt_quote_mint_url_index ON melt_quote(mint_url);
CREATE INDEX IF NOT EXISTS transactions_mint_url_index ON transactions(mint_url);
