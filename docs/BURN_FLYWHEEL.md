# Press Sovereign Treasury Flywheel (Yearly Burn)

## Objective
Create a predictable, investor-grade scarcity mechanism that is:
- transparent (on-chain)
- controllable via governance (with safety rails)
- non-extractive (does not harm normal users)
- aligned with treasury solvency

## Revenue inflows (examples)
- outlet creation fees
- outlet token listing fees (tiers)
- liquidity routing fees (paid in PRESS)
- dispute filing bonds (failed disputes)
- enterprise AI verification API (paid in PRESS)

## Flywheel design
### A) Accumulate
Protocol fees flow into Treasury in PRESS and/or stable assets.

### B) Safety buffer
Treasury maintains a mandatory reserve ratio (configurable) to avoid insolvency during low volume.

### C) Burn window
Once per year:
- Treasury buys PRESS (if part of holdings are in stable assets)
- Burns a governed portion of PRESS held above reserve

### D) Grant funding
Grants must be paid in PRESS and are constrained by:
- treasury reserve rules
- governance approvals
- anti-drain caps

## Guardrails
- Max burn % per period
- Minimum treasury reserve (absolute + %)
- Emergency pause vote (council multisig + community threshold)
- All actions emit events for explorers

## Explorer requirements
Events must be emitted for:
- FeeCollected
- TreasuryReserveUpdated
- BurnExecuted
- GrantPaid
