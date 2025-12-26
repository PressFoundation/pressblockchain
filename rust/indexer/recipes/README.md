# Press Index Recipes (RR23)

This folder defines *subgraphless* decoding recipes. Each recipe:
- lists contract addresses (resolved from /state/deploy.json)
- lists event signatures (topic0)
- defines normalized table targets

RR23 ships placeholders; RR24 will implement the poller/subscriber and decoding.

## Planned tables
- proposals
- votes
- council_endorsements
- param_changes
- oracle_reports (from oracle service)
- outlets
- court_cases
- marketplace_listings
