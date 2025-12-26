# Source Role (Optional KYC-like but decentralized)

## Goal
Enable a verified "Source" role that journalists/outlets can attach to articles without revealing identity publicly.

Sources:
- can be individuals or businesses
- obtain role via an optional verification workflow (Press Pass identity + attestations)
- can opt into:
  - being public (discoverable profile)
  - being private (only cryptographic proofs; Source Secrecy Vault integration)

## How Sources get used
- A journalist/outlet requests a Source on a specific article.
- A Source can:
  - accept (attach to the article)
  - decline
- A Source can also join an outlet as an ongoing Source.

## Revenue model
- Each article with an attached Source allocates a % split (configurable by article policy):
  - default: 5–15% of net article revenue (tips + licensing + syndication)
- Sources have a searchable on-chain registry:
  - SourceRegistry (role status + metadata hash)
  - SourcePool (aggregate accounting for earnings)

## Anti-abuse rules
- Source cannot seize article rights (rights remain with primary author/outlet)
- Source revenue share is immutable once article is finalized (prevents coercive edits)
- Source role requires:
  - verification bond (refundable after inactivity period)
  - activity requirements to retain “active” status
- Events emitted for explorers:
  - SourceRegistered
  - SourceAttached
  - SourceEarningsAccrued
  - SourceEarningsWithdrawn
