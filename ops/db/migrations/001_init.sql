CREATE TABLE IF NOT EXISTS contracts (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  address TEXT NOT NULL,
  chain_id BIGINT NOT NULL,
  deployed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS contracts_name_chain_uq ON contracts(name, chain_id);

CREATE TABLE IF NOT EXISTS chain_metrics (
  id SERIAL PRIMARY KEY,
  key TEXT NOT NULL,
  value JSONB NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS chain_metrics_key_idx ON chain_metrics(key);

CREATE TABLE IF NOT EXISTS outlets (
  outlet_id BIGINT PRIMARY KEY,
  owner TEXT NOT NULL,
  name TEXT NOT NULL,
  domain TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS outlet_tokens (
  token_address TEXT PRIMARY KEY,
  owner TEXT NOT NULL,
  name TEXT NOT NULL,
  symbol TEXT NOT NULL,
  supply NUMERIC NOT NULL,
  deployed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS articles (
  article_id BIGINT PRIMARY KEY,
  outlet_id BIGINT NOT NULL,
  author TEXT NOT NULL,
  uri TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS proposals (
  proposal_id BIGINT PRIMARY KEY,
  proposer TEXT NOT NULL,
  proposal_type TEXT NOT NULL,
  title TEXT NOT NULL,
  description_uri TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  ends_at TIMESTAMPTZ NOT NULL,
  fee_paid NUMERIC NOT NULL
);

CREATE TABLE IF NOT EXISTS court_cases (
  case_id BIGINT PRIMARY KEY,
  outlet_id BIGINT NOT NULL,
  filed_by TEXT NOT NULL,
  case_type TEXT NOT NULL,
  evidence_uri TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS params (
  key TEXT PRIMARY KEY,
  value NUMERIC NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS bonds (
  account TEXT NOT NULL,
  role TEXT NOT NULL,
  amount NUMERIC NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (account, role)
);

CREATE TABLE IF NOT EXISTS proposal_votes (
  proposal_id BIGINT NOT NULL,
  voter TEXT NOT NULL,
  support BOOLEAN NOT NULL,
  weight NUMERIC NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (proposal_id, voter)
);

CREATE TABLE IF NOT EXISTS multisig_txs (
  tx_id BIGINT PRIMARY KEY,
  target TEXT NOT NULL,
  value NUMERIC NOT NULL,
  approvals BIGINT NOT NULL,
  status TEXT NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
