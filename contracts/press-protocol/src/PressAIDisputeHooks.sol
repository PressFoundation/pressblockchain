// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * PressAIDisputeHooks (RR182 rebuild)
 * Non-censoring dispute records produced by AI / oracle systems.
 * Escalation to Court occurs via Court module.
 */
contract PressAIDisputeHooks {
    struct Dispute {
        bytes32 id;
        bytes32 articleId;
        uint64 createdAt;
        uint8 severity;
        uint16 confidenceBps;
        bytes32 modelHash;
        bytes32 evidenceHash;
        bool escalated;
        bool resolved;
        bytes32 resolutionHash;
    }

    mapping(bytes32 => Dispute) public disputes;

    event DisputeFlaggedAI(bytes32 indexed disputeId, bytes32 indexed articleId, uint8 severity, uint16 confidenceBps, bytes32 modelHash, bytes32 evidenceHash);
    event DisputeEscalated(bytes32 indexed disputeId, bytes32 indexed articleId, bytes32 courtCaseHash);
    event DisputeResolved(bytes32 indexed disputeId, bytes32 resolutionHash);

    function flagDisputeAI(
        bytes32 articleId,
        uint8 severity,
        uint16 confidenceBps,
        bytes32 modelHash,
        bytes32 evidenceHash
    ) external returns (bytes32 disputeId) {
        require(articleId != bytes32(0), "article");
        require(severity >= 1 && severity <= 10, "sev");
        require(confidenceBps <= 10000, "conf");

        disputeId = keccak256(abi.encode(articleId, severity, confidenceBps, modelHash, evidenceHash, block.timestamp, block.chainid));
        disputes[disputeId] = Dispute({
            id: disputeId,
            articleId: articleId,
            createdAt: uint64(block.timestamp),
            severity: severity,
            confidenceBps: confidenceBps,
            modelHash: modelHash,
            evidenceHash: evidenceHash,
            escalated: false,
            resolved: false,
            resolutionHash: bytes32(0)
        });

        emit DisputeFlaggedAI(disputeId, articleId, severity, confidenceBps, modelHash, evidenceHash);
    }

    function escalateToCourt(bytes32 disputeId, bytes32 courtCaseHash) external {
        Dispute storage D = disputes[disputeId];
        require(D.id != bytes32(0), "unknown");
        require(!D.escalated, "done");
        D.escalated = true;
        emit DisputeEscalated(disputeId, D.articleId, courtCaseHash);
    }

    function resolve(bytes32 disputeId, bytes32 resolutionHash) external {
        Dispute storage D = disputes[disputeId];
        require(D.id != bytes32(0), "unknown");
        require(!D.resolved, "done");
        D.resolved = true;
        D.resolutionHash = resolutionHash;
        emit DisputeResolved(disputeId, resolutionHash);
    }
}
