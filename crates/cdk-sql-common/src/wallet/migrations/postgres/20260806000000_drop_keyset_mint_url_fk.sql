-- Drop the keyset -> mint foreign key so a mint can be identified by its pubkey.
--
-- A mint promoted to the pubkey-keyed store leaves the URL-keyed `mint` table,
-- which the foreign key would reject. Keyset deletion is now explicit in
-- `remove_mint` instead of relying on ON DELETE CASCADE.
--
-- The constraint is unnamed in 1_initial.sql, so PostgreSQL auto-named it.
ALTER TABLE keyset DROP CONSTRAINT IF EXISTS keyset_mint_url_fkey;

-- Rewriting a mint's identity filters every dependent table by mint_url.
CREATE INDEX IF NOT EXISTS keyset_mint_url_index ON keyset(mint_url);
CREATE INDEX IF NOT EXISTS mint_quote_mint_url_index ON mint_quote(mint_url);
CREATE INDEX IF NOT EXISTS melt_quote_mint_url_index ON melt_quote(mint_url);
CREATE INDEX IF NOT EXISTS proof_mint_url_index ON proof(mint_url);
