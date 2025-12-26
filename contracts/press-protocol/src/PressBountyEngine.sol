// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * PressBountyEngine (RR188)
 * Bounty-style incentives paid in PRESS directly to wallets (not bonds):
 * - Vote missions (N votes within T hours)
 * - Proposal reviews
 * - Fact dispute validations
 * - Dev quests (SDK/tooling tasks)
 *
 * Canonical events only; enforcement and payout funded by treasury/pools.
 */
contract PressBountyEngine {
    struct Bounty {
        bytes32 id;
        bytes32 kind;        // e.g., "VOTE_MISSION", "DEV_QUEST"
        uint256 rewardPress;
        uint64 startAt;
        uint64 endAt;
        uint32 minActions;
        bytes32 metadataHash;
        bool active;
    }

    mapping(bytes32 => Bounty) public bounties;

    event BountyCreated(bytes32 indexed bountyId, bytes32 indexed kind, uint256 rewardPress, uint64 startAt, uint64 endAt, uint32 minActions, bytes32 metadataHash);
    event BountyCompleted(bytes32 indexed bountyId, address indexed wallet, uint32 actions, bytes32 proofHash);
    event BountyPaid(bytes32 indexed bountyId, address indexed wallet, uint256 rewardPress, bytes32 ref);
    event BountyClosed(bytes32 indexed bountyId, bytes32 reasonHash);

    function emitCreate(Bounty calldata b) external {
        bounties[b.id] = b;
        emit BountyCreated(b.id, b.kind, b.rewardPress, b.startAt, b.endAt, b.minActions, b.metadataHash);
    }

    function emitComplete(bytes32 bountyId, address wallet, uint32 actions, bytes32 proofHash) external {
        emit BountyCompleted(bountyId, wallet, actions, proofHash);
    }

    function emitPay(bytes32 bountyId, address wallet, uint256 rewardPress, bytes32 ref) external {
        emit BountyPaid(bountyId, wallet, rewardPress, ref);
    }

    function emitClose(bytes32 bountyId, bytes32 reasonHash) external {
        emit BountyClosed(bountyId, reasonHash);
    }
}
