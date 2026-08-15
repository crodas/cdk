-- A mint's URL can change; its NUT-06 public key cannot. Give the mint row a
-- surrogate id so a move is a single-column update, and resolve every caller
-- identifier (public key or URL) through mint_alias. Every alias except a
-- superseded URL is derivable from the mint rows, so a damaged index can be
-- rebuilt at the cost of old URLs no longer resolving.

ALTER TABLE keyset DROP CONSTRAINT IF EXISTS keyset_mint_url_fkey;
ALTER TABLE mint DROP CONSTRAINT mint_pkey;
ALTER TABLE mint ADD COLUMN id BIGSERIAL PRIMARY KEY;
-- mint_url is a contact address, not an identity: mint_alias decides which row
-- a URL resolves to, so no uniqueness is declared here.
ALTER TABLE mint ALTER COLUMN mint_url SET NOT NULL;

-- The previous update_mint_url moved proofs and mint quotes to a URL without
-- moving the mint row, so a wallet that ran it has rows pointing at a URL with
-- no mint. Recreate those mints rather than dropping the rows below.
INSERT INTO mint (mint_url)
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
WHERE NOT EXISTS (SELECT 1 FROM mint m WHERE m.mint_url = s.mint_url);

-- Every identifier a caller may hold resolves here. Old URLs are kept after a
-- move so stale references still resolve. Duplicate public keys (the same mint
-- added twice under different URLs) leave the alias with the lowest mint id;
-- the other row stays reachable by URL.
CREATE TABLE mint_alias (
    alias TEXT PRIMARY KEY,
    mint_id BIGINT NOT NULL REFERENCES mint(id) ON DELETE CASCADE
);
CREATE INDEX mint_alias_mint_id_index ON mint_alias(mint_id);

INSERT INTO mint_alias (alias, mint_id)
SELECT mint_url, id FROM mint ORDER BY id
ON CONFLICT (alias) DO NOTHING;

INSERT INTO mint_alias (alias, mint_id)
SELECT encode(pubkey, 'hex'), id FROM mint WHERE pubkey IS NOT NULL ORDER BY id
ON CONFLICT (alias) DO NOTHING;

ALTER TABLE keyset ADD COLUMN mint_id BIGINT;
UPDATE keyset SET mint_id = m.id FROM mint m WHERE m.mint_url = keyset.mint_url;
ALTER TABLE keyset ALTER COLUMN mint_id SET NOT NULL;
ALTER TABLE keyset ADD CONSTRAINT keyset_mint_id_fkey FOREIGN KEY (mint_id) REFERENCES mint(id) ON DELETE CASCADE;
ALTER TABLE keyset DROP COLUMN mint_url;
CREATE INDEX keyset_mint_id_index ON keyset(mint_id);

ALTER TABLE proof ADD COLUMN mint_id BIGINT;
UPDATE proof SET mint_id = m.id FROM mint m WHERE m.mint_url = proof.mint_url;
ALTER TABLE proof ALTER COLUMN mint_id SET NOT NULL;
ALTER TABLE proof ADD CONSTRAINT proof_mint_id_fkey FOREIGN KEY (mint_id) REFERENCES mint(id) ON DELETE CASCADE;
ALTER TABLE proof DROP COLUMN mint_url;
CREATE INDEX proof_mint_id_index ON proof(mint_id);

ALTER TABLE mint_quote ADD COLUMN mint_id BIGINT;
UPDATE mint_quote SET mint_id = m.id FROM mint m WHERE m.mint_url = mint_quote.mint_url;
ALTER TABLE mint_quote ALTER COLUMN mint_id SET NOT NULL;
ALTER TABLE mint_quote ADD CONSTRAINT mint_quote_mint_id_fkey FOREIGN KEY (mint_id) REFERENCES mint(id) ON DELETE CASCADE;
ALTER TABLE mint_quote DROP COLUMN mint_url;
CREATE INDEX mint_quote_mint_id_index ON mint_quote(mint_id);

-- melt_quote.mint_url was added late and is nullable, so mint_id stays nullable
-- and rows that never recorded a mint keep NULL instead of being dropped.
ALTER TABLE melt_quote ADD COLUMN mint_id BIGINT REFERENCES mint(id) ON DELETE CASCADE;
UPDATE melt_quote SET mint_id = m.id FROM mint m WHERE m.mint_url = melt_quote.mint_url;
ALTER TABLE melt_quote DROP COLUMN mint_url;
CREATE INDEX melt_quote_mint_id_index ON melt_quote(mint_id);

ALTER TABLE transactions ADD COLUMN mint_id BIGINT;
UPDATE transactions SET mint_id = m.id FROM mint m WHERE m.mint_url = transactions.mint_url;
ALTER TABLE transactions ALTER COLUMN mint_id SET NOT NULL;
ALTER TABLE transactions ADD CONSTRAINT transactions_mint_id_fkey FOREIGN KEY (mint_id) REFERENCES mint(id) ON DELETE CASCADE;
DROP INDEX IF EXISTS mint_url_index;
ALTER TABLE transactions DROP COLUMN mint_url;
CREATE INDEX transactions_mint_id_index ON transactions(mint_id);

ALTER TABLE wallet_sagas ADD COLUMN mint_id BIGINT;
UPDATE wallet_sagas SET mint_id = m.id FROM mint m WHERE m.mint_url = wallet_sagas.mint_url;
ALTER TABLE wallet_sagas ALTER COLUMN mint_id SET NOT NULL;
ALTER TABLE wallet_sagas ADD CONSTRAINT wallet_sagas_mint_id_fkey FOREIGN KEY (mint_id) REFERENCES mint(id) ON DELETE CASCADE;
DROP INDEX IF EXISTS wallet_sagas_mint_url_index;
ALTER TABLE wallet_sagas DROP COLUMN mint_url;
CREATE INDEX wallet_sagas_mint_id_index ON wallet_sagas(mint_id);
