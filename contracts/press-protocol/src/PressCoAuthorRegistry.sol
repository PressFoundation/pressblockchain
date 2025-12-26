// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * PressCoAuthorRegistry (RR186)
 * Adds a secondary co-author for a small PRESS fee (enforced by higher-level modules).
 * Revenue split is immutable 50/50; primary retains control rights.
 */
contract PressCoAuthorRegistry {
    struct CoAuthor {
        bytes32 articleId;
        address primary;
        address secondary;
        bool active;
        uint64 createdAt;
    }

    mapping(bytes32 => CoAuthor) public coauthors;

    event CoAuthorAdded(bytes32 indexed articleId, address indexed primary, address indexed secondary);
    event CoAuthorRemoved(bytes32 indexed articleId, bytes32 reasonHash);

    function addCoAuthor(bytes32 articleId, address secondary) external {
        require(articleId != bytes32(0), "article");
        require(secondary != address(0), "secondary");
        require(coauthors[articleId].articleId == bytes32(0), "exists");
        coauthors[articleId] = CoAuthor({
            articleId: articleId,
            primary: msg.sender,
            secondary: secondary,
            active: true,
            createdAt: uint64(block.timestamp)
        });
        emit CoAuthorAdded(articleId, msg.sender, secondary);
    }

    function removeCoAuthor(bytes32 articleId, bytes32 reasonHash) external {
        CoAuthor storage c = coauthors[articleId];
        require(c.articleId != bytes32(0) && c.active, "none");
        require(msg.sender == c.primary, "auth");
        c.active = false;
        emit CoAuthorRemoved(articleId, reasonHash);
    }
}
