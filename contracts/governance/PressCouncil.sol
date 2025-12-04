// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../utils/AccessRoles.sol";
import "../identity/PressID.sol";

contract PressCouncil is AccessRoles {
    struct CouncilMember {
        address member;
        uint64 joinedAt;
        uint64 expiresAt;
        bool active;
    }

    PressID public pressId;
    uint16 public maxCouncilMembers;
    uint64 public termDuration;
    address[] private _members;
    mapping(address => CouncilMember) public councilInfo;

    event CouncilConfigured(uint16 maxMembers, uint64 termDuration);
    event CouncilMemberAdded(address indexed member, uint64 joinedAt, uint64 expiresAt);
    event CouncilMemberRemoved(address indexed member, string reason);
    event CouncilMemberExpired(address indexed member, uint64 at);

    constructor(address superAdmin, address pressId_, uint16 maxMembers, uint64 termDuration_)
        AccessRoles(superAdmin)
    {
        pressId = PressID(pressId_);
        maxCouncilMembers = maxMembers;
        termDuration = termDuration_;
        emit CouncilConfigured(maxMembers, termDuration_);
    }

    function getCouncilMembers() external view returns (address[] memory) {
        return _members;
    }

    function isCouncilMember(address user) public view returns (bool) {
        return councilInfo[user].active && councilInfo[user].expiresAt > block.timestamp;
    }

    function addCouncilMember(address user)
        external
        onlyRole(SUPER_ADMIN_ROLE)
    {
        require(!isCouncilMember(user), "Already council");
        require(_members.length < maxCouncilMembers, "Council full");
        uint64 nowTs = uint64(block.timestamp);
        uint64 expiry = nowTs + termDuration;
        councilInfo[user] = CouncilMember({
            member: user,
            joinedAt: nowTs,
            expiresAt: expiry,
            active: true
        });
        _members.push(user);
        _grantRole(PRESS_COUNCIL_ROLE, user);
        emit CouncilMemberAdded(user, nowTs, expiry);
    }

    function removeCouncilMember(address user, string calldata reason)
        external
        onlyRole(SUPER_ADMIN_ROLE)
    {
        CouncilMember storage c = councilInfo[user];
        if (!c.active) return;
        c.active = false;
        c.expiresAt = uint64(block.timestamp);
        _revokeRole(PRESS_COUNCIL_ROLE, user);
        _removeFromArray(user);
        emit CouncilMemberRemoved(user, reason);
    }

    function rotateExpired() external {
        uint256 len = _members.length;
        for (uint256 i = 0; i < len; ) {
            address m = _members[i];
            CouncilMember storage c = councilInfo[m];
            if (c.active && c.expiresAt <= block.timestamp) {
                c.active = false;
                _revokeRole(PRESS_COUNCIL_ROLE, m);
                emit CouncilMemberExpired(m, c.expiresAt);
                _members[i] = _members[len - 1];
                _members.pop();
                len--;
                continue;
            }
            unchecked { i++; }
        }
    }

    function _removeFromArray(address user) internal {
        uint256 len = _members.length;
        for (uint256 i = 0; i < len; i++) {
            if (_members[i] == user) {
                _members[i] = _members[len - 1];
                _members.pop();
                break;
            }
        }
    }
}
