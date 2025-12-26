-- RR9: track council removal reasons and timestamps
ALTER TABLE council_members
  ADD COLUMN IF NOT EXISTS removal_reason TEXT,
  ADD COLUMN IF NOT EXISTS removed_at TIMESTAMPTZ;
