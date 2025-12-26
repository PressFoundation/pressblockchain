Press Protocol Contracts (Foundry)
- Deploys canonical, explorer-friendly fee + governance signal emitters.
- Installer runs Foundry in a container; no host installation required.

Contracts:
- PressTreasury: receives protocol fees
- PressFeeRouter: enforces fixed-fee contexts and emits FeePaid events
- PressGovernanceSignals: emits standard events for proposals/votes/closures

These events are designed to be trivially indexed by Blockscout/custom explorers.
