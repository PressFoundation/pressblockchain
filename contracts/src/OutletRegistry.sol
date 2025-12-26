// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IPressParameters {
    function params(bytes32 key) external view returns (uint256);
}

interface IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

contract OutletRegistry {
    event OutletCreated(bytes32 indexed outletId, address indexed owner, string name, string domain, uint256 bondPaid, uint256 feePaid);
    event DomainVerified(bytes32 indexed outletId, string domain, uint8 proofType, bytes32 proofHash, address indexed verifier);
    event OutletRoleGranted(bytes32 indexed outletId, address indexed account, bytes32 indexed role);
    event OutletRoleRevoked(bytes32 indexed outletId, address indexed account, bytes32 indexed role);

    bytes32 public constant ROLE_MANAGER = keccak256("OUTLET_MANAGER");
    bytes32 public constant ROLE_EDITOR  = keccak256("OUTLET_EDITOR");
    bytes32 public constant ROLE_WRITER  = keccak256("OUTLET_WRITER");
    bytes32 public constant ROLE_ANALYST = keccak256("OUTLET_ANALYST");
    bytes32 public constant ROLE_PHOTO   = keccak256("OUTLET_PHOTOGRAPHER");

    address public immutable pressToken;
    address public immutable treasury;
    IPressParameters public immutable pressParams;

    struct Outlet {
        address owner;
        string name;
        string domain;
        uint256 createdAt;
        uint256 bond;
        bool exists;
    }

    mapping(bytes32 => Outlet) public outlets;
    mapping(bytes32 => mapping(address => mapping(bytes32 => bool))) public hasRole;

    mapping(bytes32 => address[]) private _members;
    mapping(bytes32 => mapping(address => uint256)) private _memberIndexPlus1;
    bytes32 public constant ROLE_MEMBER = keccak256("OUTLET_MEMBER");
 // outletId => addr => role => bool

    constructor(address _pressToken, address _treasury, address _pressParams) {
        pressToken = _pressToken;
        treasury = _treasury;
        pressParams = IPressParameters(_pressParams);
    }

    function outletIdFromDomain(string memory domain) public pure returns (bytes32) {
        return keccak256(abi.encodePacked(domain));
    }

    function createOutlet(string calldata name, string calldata domain) external returns (bytes32) {
        require(bytes(name).length > 1, "NAME_REQUIRED");
        require(bytes(domain).length > 3, "DOMAIN_REQUIRED");

        bytes32 id = outletIdFromDomain(domain);
        require(!outlets[id].exists, "OUTLET_EXISTS");

        uint256 fee = pressParams.params(keccak256("outlet_create_fee"));
        uint256 bond = pressParams.params(keccak256("outlet_bond_min"));

        // PRESS-only economics
        if (fee > 0) require(IERC20(pressToken).transferFrom(msg.sender, treasury, fee), "FEE_TRANSFER_FAIL");
        if (bond > 0) require(IERC20(pressToken).transferFrom(msg.sender, address(this), bond), "BOND_TRANSFER_FAIL");

        outlets[id] = Outlet({
            owner: msg.sender,
            name: name,
            domain: domain,
            createdAt: block.timestamp,
            bond: bond,
            exists: true
        });

        // bootstrap roles
        hasRole[id][msg.sender][ROLE_MANAGER] = true;

        emit OutletCreated(id, msg.sender, name, domain, bond, fee);
        emit OutletRoleGranted(id, msg.sender, ROLE_MANAGER);
        return id;
    }

    modifier onlyManager(bytes32 outletId) {
        require(outlets[outletId].exists, "NO_OUTLET");
        require(hasRole[outletId][msg.sender][ROLE_MANAGER] || outlets[outletId].owner == msg.sender, "NOT_MANAGER");
        _;
    }

    function grantRole(bytes32 outletId, address account, bytes32 role) external onlyManager(outletId) {
        hasRole[outletId][account][role] = true;
        _ensureMember(outletId, account);
        emit OutletRoleGranted(outletId, account, role);
    }

    function revokeRole(bytes32 outletId, address account, bytes32 role) external onlyManager(outletId) {
        hasRole[outletId][account][role] = false;
        emit OutletRoleRevoked(outletId, account, role);
    }
/// @notice Records a verifiable domain-ownership proof for explorer/UI consumption.
/// @dev On-chain does not resolve DNS; this is an immutable attestation anchored by tx.
/// proofType: 1 = DNS TXT token, 2 = Wallet signature hash, 3 = Other
function verifyDomain(bytes32 outletId, string calldata domain, uint8 proofType, bytes32 proofHash) external {
    require(outlets[outletId].exists, "OUTLET_NOT_FOUND");
    require(bytes(domain).length > 0, "DOMAIN_EMPTY");
    emit DomainVerified(outletId, domain, proofType, proofHash, msg.sender);
}

function membersOf(bytes32 outletId) external view returns (address[] memory) {
    return _members[outletId];
}

function memberCount(bytes32 outletId) external view returns (uint256) {
    return _members[outletId].length;
}

function _ensureMember(bytes32 outletId, address account) internal {
    if (_memberIndexPlus1[outletId][account] == 0) {
        _members[outletId].push(account);
        _memberIndexPlus1[outletId][account] = _members[outletId].length; // index+1
    }
}

function _tryRemoveMember(bytes32 outletId, address account) internal {
    uint256 idxp = _memberIndexPlus1[outletId][account];
    if (idxp == 0) return;
    // Remove only if account has no remaining roles
    // Lightweight check: if any role mapping true for common roles, keep. Managers can manually clean up via future maintenance.
    if (hasRole[outletId][account][ROLE_MEMBER]) return;
    // If more roles are introduced, indexer can keep ROLE_MEMBER on for anyone active.
    uint256 idx = idxp - 1;
    uint256 last = _members[outletId].length - 1;
    if (idx != last) {
        address lastAddr = _members[outletId][last];
        _members[outletId][idx] = lastAddr;
        _memberIndexPlus1[outletId][lastAddr] = idx + 1;
    }
    _members[outletId].pop();
    _memberIndexPlus1[outletId][account] = 0;
}

}
