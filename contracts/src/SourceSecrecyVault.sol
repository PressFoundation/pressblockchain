// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice Proof of source existence without exposure.
/// Stores encrypted attestations + commitments and optional court-unlock metadata.
contract SourceSecrecyVault {
    struct SourceProof {
        bytes32 articleId;
        bytes32 commitment;   // hash of encrypted blob + salt
        string  encryptedUri; // pointer to encrypted content (IPFS/S3/Arweave), never plaintext
        uint64  createdAt;
        bool    courtUnlockable;
        bytes32 unlockPolicyHash; // hash describing unlock policy
    }

    event SourceProofAdded(bytes32 indexed articleId, address indexed submitter, bytes32 commitment, string encryptedUri, bool courtUnlockable, bytes32 unlockPolicyHash);

    mapping(bytes32 => SourceProof[]) public proofsByArticle;

    function addSourceProof(
        bytes32 articleId,
        bytes32 commitment,
        string calldata encryptedUri,
        bool courtUnlockable,
        bytes32 unlockPolicyHash
    ) external {
        require(articleId != bytes32(0), "BAD_ARTICLE");
        require(commitment != bytes32(0), "BAD_COMMIT");
        require(bytes(encryptedUri).length > 0, "BAD_URI");
        proofsByArticle[articleId].push(SourceProof({
            articleId: articleId,
            commitment: commitment,
            encryptedUri: encryptedUri,
            createdAt: uint64(block.timestamp),
            courtUnlockable: courtUnlockable,
            unlockPolicyHash: unlockPolicyHash
        }));
        emit SourceProofAdded(articleId, msg.sender, commitment, encryptedUri, courtUnlockable, unlockPolicyHash);
    }

    function proofCount(bytes32 articleId) external view returns (uint256) {
        return proofsByArticle[articleId].length;
    }
}
