// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * PressSourceRegistry (RR186)
 * Optional Press Pass (KYC-like) gating for Source role.
 * Supports: public profile hashes, discovery pool, and article/outlet attachments.
 * No censorship: Sources can only be attached by consented requests.
 */
contract PressSourceRegistry {
    struct SourceProfile {
        address wallet;
        bool active;
        bool kycVerified;
        bytes32 profileHash;     // IPFS/Arweave or offchain bundle hash
        bytes32 tagsHash;        // public discoverability tags hash
        uint64 createdAt;
    }

    struct SourceAttachment {
        bytes32 id;
        bytes32 articleId;
        bytes32 outletId;
        address sourceWallet;
        uint16 revenueShareBps;  // share of article revenue for this source
        bool active;
        uint64 createdAt;
    }

    mapping(address => SourceProfile) public sources;
    mapping(bytes32 => SourceAttachment) public attachments;

    event SourceRegistered(address indexed sourceWallet, bool kycVerified, bytes32 profileHash, bytes32 tagsHash);
    event SourceUpdated(address indexed sourceWallet, bool kycVerified, bytes32 profileHash, bytes32 tagsHash, bool active);
    event SourceAttachmentRequested(bytes32 indexed requestId, bytes32 indexed articleId, bytes32 indexed outletId, address requester, address sourceWallet);
    event SourceAttachmentAccepted(bytes32 indexed attachmentId, bytes32 indexed articleId, bytes32 indexed outletId, address sourceWallet, uint16 revenueShareBps);
    event SourceAttachmentRevoked(bytes32 indexed attachmentId, bytes32 reasonHash);

    function registerSource(bool kycVerified, bytes32 profileHash, bytes32 tagsHash) external {
        sources[msg.sender] = SourceProfile({
            wallet: msg.sender,
            active: true,
            kycVerified: kycVerified,
            profileHash: profileHash,
            tagsHash: tagsHash,
            createdAt: uint64(block.timestamp)
        });
        emit SourceRegistered(msg.sender, kycVerified, profileHash, tagsHash);
    }

    function updateSource(bool kycVerified, bytes32 profileHash, bytes32 tagsHash, bool active) external {
        SourceProfile storage s = sources[msg.sender];
        require(s.wallet != address(0), "not_registered");
        s.kycVerified = kycVerified;
        s.profileHash = profileHash;
        s.tagsHash = tagsHash;
        s.active = active;
        emit SourceUpdated(msg.sender, kycVerified, profileHash, tagsHash, active);
    }

    function requestAttachment(bytes32 articleId, bytes32 outletId, address sourceWallet) external returns (bytes32 requestId) {
        require(articleId != bytes32(0) || outletId != bytes32(0), "target");
        require(sourceWallet != address(0), "source");
        requestId = keccak256(abi.encode(articleId, outletId, msg.sender, sourceWallet, block.timestamp, block.chainid));
        emit SourceAttachmentRequested(requestId, articleId, outletId, msg.sender, sourceWallet);
    }

    function acceptAttachment(bytes32 requestId, bytes32 articleId, bytes32 outletId, uint16 revenueShareBps) external returns (bytes32 attachmentId) {
        require(revenueShareBps <= 5000, "share"); // cap 50% by default
        // Only source can accept
        require(msg.sender != address(0), "auth");
        attachmentId = keccak256(abi.encode(requestId, msg.sender, articleId, outletId, revenueShareBps, block.chainid));
        attachments[attachmentId] = SourceAttachment({
            id: attachmentId,
            articleId: articleId,
            outletId: outletId,
            sourceWallet: msg.sender,
            revenueShareBps: revenueShareBps,
            active: true,
            createdAt: uint64(block.timestamp)
        });
        emit SourceAttachmentAccepted(attachmentId, articleId, outletId, msg.sender, revenueShareBps);
    }

    function revokeAttachment(bytes32 attachmentId, bytes32 reasonHash) external {
        SourceAttachment storage a = attachments[attachmentId];
        require(a.id != bytes32(0), "unknown");
        require(msg.sender == a.sourceWallet, "auth");
        a.active = false;
        emit SourceAttachmentRevoked(attachmentId, reasonHash);
    }
}
