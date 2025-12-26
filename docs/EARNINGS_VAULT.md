# Journalist Earnings Vault (PRESS-native payroll)

## Purpose
Make PRESS the working capital of journalism:
- tips
- royalties
- syndication revenue
- truth escrow rewards

## Vault rules (high level)
- Each wallet can have an EarningsVault.
- Vault accumulates receivables by category.
- Withdrawals go directly to wallet balance (not bond).
- Optional auto-split to:
  - co-author
  - outlet pool
  - source share
  - treasury cut

## Explorer requirements
All vault activity emits events:
- VaultAccrued(category, amount, ref)
- VaultWithdrawn(amount, to)
- VaultSplitExecuted(primary, secondary, outlet, source, treasury)
