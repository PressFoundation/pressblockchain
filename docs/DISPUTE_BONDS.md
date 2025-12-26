# Dispute Filing Bonds

## Goal
Allow challenges while preventing bad-faith spam:
- challenge requires a PRESS bond
- failed challenge: bond burned or routed to treasury
- successful challenge: bond redistributed to reviewers / correction pool

## Mechanics
- bond amount scales with:
  - article impact score (reach, tips, syndication volume)
  - challenger role (discounts for verified roles)
- dispute auto-closes after time window
- all outcomes stored on-chain with reason code

## Events for explorers
- DisputeOpened
- DisputeResolved
- BondSlashed / BondReturned
