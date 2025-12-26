// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * PressGovernanceFeeRouter (RR187)
 * Canonical events for governance fee payments:
 * - pay-to-vote (flat PRESS fee per vote, configurable by governance)
 * - proposal submission fee + partial refund on approval
 *
 * Transfer enforcement is implemented by downstream vaults/adapters; this is the
 * canonical on-chain event layer for Blockscout/custom explorers.
 */
contract PressGovernanceFeeRouter {
    event VoteFeePaid(bytes32 indexed proposalId, address indexed voter, uint256 feePress, bytes32 ref);
    event ProposalFeePaid(bytes32 indexed proposalId, address indexed proposer, uint256 feePress, bytes32 ref);
    event ProposalFeeRefunded(bytes32 indexed proposalId, address indexed proposer, uint256 refundPress, uint16 refundPct, bytes32 ref);

    function emitVoteFee(bytes32 proposalId, address voter, uint256 feePress, bytes32 ref) external {
        emit VoteFeePaid(proposalId, voter, feePress, ref);
    }

    function emitProposalFee(bytes32 proposalId, address proposer, uint256 feePress, bytes32 ref) external {
        emit ProposalFeePaid(proposalId, proposer, feePress, ref);
    }

    function emitProposalRefund(bytes32 proposalId, address proposer, uint256 refundPress, uint16 refundPct, bytes32 ref) external {
        emit ProposalFeeRefunded(proposalId, proposer, refundPress, refundPct, ref);
    }
}
