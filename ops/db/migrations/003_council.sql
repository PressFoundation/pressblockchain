-- RR7: council membership lifecycle
CREATE TABLE IF NOT EXISTS council_members (
  member TEXT PRIMARY KEY,
  active BOOLEAN NOT NULL,
  term_start TIMESTAMPTZ NOT NULL,
  term_end TIMESTAMPTZ NOT NULL,
  last_activity TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
