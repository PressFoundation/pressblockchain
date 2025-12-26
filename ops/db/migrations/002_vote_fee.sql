-- RR6: track voting fee (if emitted) and proposal lifecycle events
ALTER TABLE proposal_votes
  ADD COLUMN IF NOT EXISTS fee_paid NUMERIC NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS proposal_lifecycle (
  id SERIAL PRIMARY KEY,
  proposal_id BIGINT NOT NULL,
  event_type TEXT NOT NULL, -- finalized|executed
  data JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
