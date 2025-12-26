// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/// @notice Emits canonical events for proposals/court/votes so Blockscout/custom explorers can render.
/// State is kept minimal; authoritative state lives in your L1 modules later.
contract PressGovernanceSignals {
    event ProposalCreated(uint256 indexed id, address indexed proposer, bytes32 indexed kind, uint256 feePaid);
    event ProposalVoted(uint256 indexed id, address indexed voter, uint8 support, uint256 feePaid);
    event ProposalClosed(uint256 indexed id, uint8 outcome);

    function emitProposalCreated(uint256 id, address proposer, bytes32 kind, uint256 feePaid) external {
        emit ProposalCreated(id, proposer, kind, feePaid);
    }
    function emitProposalVoted(uint256 id, address voter, uint8 support, uint256 feePaid) external {
        emit ProposalVoted(id, voter, support, feePaid);
    }
    function emitProposalClosed(uint256 id, uint8 outcome) external {
        emit ProposalClosed(id, outcome);
    }
}
