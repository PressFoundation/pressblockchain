// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * @title PressGovernanceHub
 * @notice Event-centric governance hub to register proposals in a uniform way for explorers and indexers.
 *         Core vote logic may live elsewhere; this hub provides canonical proposal IDs and metadata hashes.
 *
 * Design:
 * - Deterministic proposalHash (keccak256 of inputs)
 * - Stores minimal metadata for lookups
 * - Emits a single canonical event for Blockscout/custom explorers
 */
contract PressGovernanceHub {
    struct Proposal {
        address proposer;
        uint64 createdAt;
        uint64 votingEndsAt;
        bytes32 proposalHash;
        uint8 kind; // 1=OutletMint, 2=ParamChange, 3=Upgrade, 4=Grant, ...
        uint256 feePaidPress;
        bool exists;
    }

    mapping(bytes32 => Proposal) public proposals;

    event ProposalRegistered(
        bytes32 indexed proposalId,
        address indexed proposer,
        uint8 indexed kind,
        bytes32 proposalHash,
        uint64 votingEndsAt,
        uint256 feePaidPress
    );

    function registerProposal(
        uint8 kind,
        bytes32 proposalHash,
        uint64 votingEndsAt,
        uint256 feePaidPress
    ) external returns (bytes32 proposalId) {
        // proposalId is deterministic and explorer friendly
        proposalId = keccak256(abi.encode(msg.sender, kind, proposalHash, votingEndsAt, feePaidPress, block.chainid));
        require(!proposals[proposalId].exists, "exists");
        proposals[proposalId] = Proposal({
            proposer: msg.sender,
            createdAt: uint64(block.timestamp),
            votingEndsAt: votingEndsAt,
            proposalHash: proposalHash,
            kind: kind,
            feePaidPress: feePaidPress,
            exists: true
        });
        emit ProposalRegistered(proposalId, msg.sender, kind, proposalHash, votingEndsAt, feePaidPress);
    }
}
