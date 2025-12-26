// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

interface IParamStore {
    function u256(bytes32 key) external view returns (uint256);
}
interface IBondVault {
    function bonded(address account, bytes32 role) external view returns (uint256);
    function lastActivity(address account) external view returns (uint64);
    function touchActivity(address account) external;
}

contract CouncilRegistry {
    uint256 public constant MAX_COUNCIL = 195;
    bytes32 public constant ROLE_COUNCIL_BOND = keccak256("COUNCIL_BOND");

    // Param keys
    bytes32 public constant P_COUNCIL_TERM_SECS = keccak256("council_term_secs");
    bytes32 public constant P_COUNCIL_MIN_BOND = keccak256("council_min_bond");
    bytes32 public constant P_COUNCIL_MAX_INACTIVITY_SECS = keccak256("council_max_inactivity_secs");

    struct Member {
        bool active;
        uint64 termStart;
        uint64 termEnd;
        uint64 lastActivity; // mirrored snapshot for explorers
    }

    mapping(address => Member) public members;
    address[] public memberList;

    address public councilMultisig;
    IParamStore public params;
    IBondVault public bondVault;

    event Config(address indexed councilMultisig, address indexed params, address indexed bondVault);
    event MemberAdded(address indexed member, uint64 termStart, uint64 termEnd);
    event MemberRemoved(address indexed member, string reason);
    event MultisigOwnerSynced(address indexed owner, bool added);
    event PruneAttempt(address indexed member, bool removed, string reason);
    event MemberActivity(address indexed member, uint64 ts);

    modifier onlyCouncilMultisig() {
        require(msg.sender == councilMultisig, "COUNCIL_ONLY");
        _;
    }

    constructor(address _councilMultisig, address _params, address _bondVault) {
        councilMultisig = _councilMultisig;
        params = IParamStore(_params);
        bondVault = IBondVault(_bondVault);
        emit Config(_councilMultisig, _params, _bondVault);
    }

    function councilCount() external view returns (uint256) { return memberList.length; }
    function memberAt(uint256 i) external view returns (address) { return memberList[i]; }
    function activeCount() external view returns (uint256) { return memberList.length; }
    function isCouncil(address a) external view returns (bool) { return members[a].active; }

    function _termWindow() internal view returns (uint64 start, uint64 end) {
        uint64 s = uint64(block.timestamp);
        uint256 t = params.u256(P_COUNCIL_TERM_SECS);
        if (t == 0) t = 180 days; // default 6 months
        uint64 e = s + uint64(t);
        return (s, e);
    }

    function _requireBond(address candidate) internal view {
        uint256 minBond = params.u256(P_COUNCIL_MIN_BOND);
        require(bondVault.bonded(candidate, ROLE_COUNCIL_BOND) >= minBond, "BOND_REQUIRED");
    }

    function addMember(address candidate) external onlyCouncilMultisig {
        require(candidate != address(0), "ADDR");
        require(!members[candidate].active, "ALREADY");
        require(memberList.length < MAX_COUNCIL, "MAX_195");
        _requireBond(candidate);

        (uint64 s, uint64 e) = _termWindow();
        uint64 la = bondVault.lastActivity(candidate);

        members[candidate] = Member({active: true, termStart: s, termEnd: e, lastActivity: la});
        memberList.push(candidate);

        emit MemberAdded(candidate, s, e);
    }

    function removeMember(address member, string calldata reason) external onlyCouncilMultisig {
        if (!members[member].active) return;
        members[member].active = false;

        // compact array
        for (uint256 i=0;i<memberList.length;i++) {
            if (memberList[i] == member) {
                memberList[i] = memberList[memberList.length-1];
                memberList.pop();
                break;
            }
        }
        emit MemberRemoved(member, reason);
    }

    function touchMemberActivity(address member) external {
        // Anyone can emit activity, but BondVault is the real source of truth;
        // we mirror latest for explorer convenience.
        uint64 la = bondVault.lastActivity(member);
        members[member].lastActivity = la;
        emit MemberActivity(member, la);
    }

    
function memberReason(address member) public view returns (string memory) {
    Member memory m = members[member];
    if (!m.active) return "not_active";
    if (block.timestamp > m.termEnd) return "term_expired";
    uint256 maxInact = params.u256(P_COUNCIL_MAX_INACTIVITY_SECS);
    if (maxInact == 0) maxInact = 30 days;
    uint64 la = bondVault.lastActivity(member);
    if (la > 0 && block.timestamp > uint256(la) + maxInact) return "inactive";
    uint256 minBond = params.u256(P_COUNCIL_MIN_BOND);
    if (bondVault.bonded(member, ROLE_COUNCIL_BOND) < minBond) return "bond_low";
    return "ok";
}

function removeIfIneligible(address member) external onlyCouncilMultisig {
    string memory why = memberReason(member);
    if (keccak256(bytes(why)) == keccak256(bytes("ok"))) {
        emit PruneAttempt(member, false, "ok");
        return;
    }
    _remove(member, why);
    emit PruneAttempt(member, true, why);
}

function pruneBatch(address[] calldata members_) external onlyCouncilMultisig {
    for (uint256 i=0;i<members_.length;i++) {
        // best-effort prune
        string memory why = memberReason(members_[i]);
        if (keccak256(bytes(why)) != keccak256(bytes("ok"))) {
            _remove(members_[i], why);
            emit PruneAttempt(members_[i], true, why);
        }
    }
}

function _remove(address member, string memory reason) internal {
    if (!members[member].active) return;
    members[member].active = false;
    for (uint256 i=0;i<memberList.length;i++) {
        if (memberList[i] == member) {
            memberList[i] = memberList[memberList.length-1];
            memberList.pop();
            break;
        }
    }
    emit MemberRemoved(member, reason);
}

// Keep CouncilMultisig owners synchronized with CouncilRegistry membership (prevents drift).
function syncMultisigOwners(address multisig, address[] calldata addOwners, address[] calldata removeOwners) external onlyCouncilMultisig {
    for (uint256 i=0;i<addOwners.length;i++) {
        (bool ok,) = multisig.call(abi.encodeWithSignature("addOwner(address)", addOwners[i]));
        require(ok, "ADD_OWNER_FAIL");
        emit MultisigOwnerSynced(addOwners[i], true);
    }
    for (uint256 i=0;i<removeOwners.length;i++) {
        (bool ok2,) = multisig.call(abi.encodeWithSignature("removeOwner(address)", removeOwners[i]));
        require(ok2, "REMOVE_OWNER_FAIL");
        emit MultisigOwnerSynced(removeOwners[i], false);
    }
}

function memberEligible(address member) external view returns (bool ok, string memory why) {
        Member memory m = members[member];
        if (!m.active) return (false, "not_active");
        if (block.timestamp > m.termEnd) return (false, "term_expired");

        uint256 maxInact = params.u256(P_COUNCIL_MAX_INACTIVITY_SECS);
        if (maxInact == 0) maxInact = 30 days;
        uint64 la = bondVault.lastActivity(member);
        if (la > 0 && block.timestamp > uint256(la) + maxInact) return (false, "inactive");
        return (true, "ok");
    }
}
