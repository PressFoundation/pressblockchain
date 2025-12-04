// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../utils/AccessRoles.sol";

contract PressID is AccessRoles {
    struct Profile {
        bool exists;
        string handle;
        string metadataURI;
        uint256 reputation;
        uint256 trustScore;
        uint256 lastUpdated;
    }

    mapping(address => Profile) private _profiles;

    event ProfileCreated(address indexed user, string handle, string metadataURI);
    event ProfileUpdated(address indexed user, string handle, string metadataURI);
    event TrustAdjusted(address indexed user, int256 delta, uint256 newTrust);

    constructor(address superAdmin) AccessRoles(superAdmin) {}

    function createOrUpdateProfile(
        string calldata handle,
        string calldata metadataURI
    ) external {
        Profile storage p = _profiles[msg.sender];
        if (!p.exists) {
            p.exists = true;
            p.reputation = 1;
            p.trustScore = 1;
            p.lastUpdated = block.timestamp;
            p.handle = handle;
            p.metadataURI = metadataURI;
            emit ProfileCreated(msg.sender, handle, metadataURI);
        } else {
            p.handle = handle;
            p.metadataURI = metadataURI;
            p.lastUpdated = block.timestamp;
            emit ProfileUpdated(msg.sender, handle, metadataURI);
        }
    }

    function adjustTrust(address user, int256 delta)
        external
        onlyRole(TRUST_ENGINE_ROLE)
    {
        Profile storage p = _profiles[user];
        require(p.exists, "No profile");
        int256 next = int256(p.trustScore) + delta;
        if (next < 0) next = 0;
        p.trustScore = uint256(next);
        p.lastUpdated = block.timestamp;
        emit TrustAdjusted(user, delta, p.trustScore);
    }

    function trustScoreOf(address user) external view returns (uint256) {
        return _profiles[user].trustScore;
    }
}
