-- RR10: store proposal voter counts for explorer-friendly anti-capture visibility
ALTER TABLE proposals
  ADD COLUMN IF NOT EXISTS voter_count BIGINT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS for_voter_count BIGINT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS against_voter_count BIGINT NOT NULL DEFAULT 0;
