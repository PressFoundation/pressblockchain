# Explorer schema guarantees (Blockscout + custom explorers)

## Objective
All core Press data must be machine-readable via:
- standard EVM event logs
- standard contract calls
- stable event signatures

This enables Blockscout and custom explorers to display:
- outlets
- articles
- votes
- court cases
- proposals
- distribution pools
- treasury flows

## Requirements
1) Every domain object has:
- a contract registry mapping id -> contract/address (where applicable)
- an event emitted on create/update/close
- a stable event signature (do not rename lightly)

2) Events must include:
- `id` (bytes32 or uint256)
- `actor` (address)
- `timestamp` (uint256)
- `metadataHash` (bytes32) for off-chain metadata pointers

## Recommended event families
- `OutletCreated`, `OutletUpdated`, `OutletRoleGranted`
- `ArticleSubmitted`, `ArticleFinalized`, `ArticleTipped`, `CoAuthorAdded`
- `VoteCast`, `VoteClosed`
- `ProposalCreated`, `ProposalExecuted`, `ProposalRejected`
- `CourtCaseOpened`, `CourtCaseEvidenceAdded`, `CourtCaseResolved`
- `FeeCollected`, `TreasuryMoved`, `BurnExecuted`
- `PoolAccrued`, `PoolDistributed`

## Indexer note
The indexer should subscribe to:
- new blocks
- all above event signatures
and store them into queryable tables for dashboards.
