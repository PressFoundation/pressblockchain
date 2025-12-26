-- RR8: track council votes on CouncilSeat proposals
CREATE TABLE IF NOT EXISTS council_votes (
  proposal_id BIGINT NOT NULL,
  council TEXT NOT NULL,
  support BOOLEAN NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (proposal_id, council)
);
