# Outlet token deployment flow (production)

## Goal
Token deployment must be:
- **Standardized**
- **Owner-controlled** (outlet wallet owns the token)
- **Verified** (deployment receipt + mandatory test transfer must pass)

## API
1) Prepare deployment calldata:
`POST /v1/outlet/token/deploy/prepare`

2) User signs and broadcasts from the outlet owner wallet.

3) Verify deploy TX:
`POST /v1/outlet/token/deploy/verify`
- returns best-effort extracted token address
- returns **mandatory** test transfer payload

4) User signs and broadcasts the test transfer.

5) Verify test TX:
`POST /v1/outlet/token/test/verify`
- only if OK is true do we mark token as usable/listable

## Why this matters
- Prevents “looks deployed” but broken tokens
- Establishes investor-grade launch hygiene
