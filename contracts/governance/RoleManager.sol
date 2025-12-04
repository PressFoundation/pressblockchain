// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../utils/AccessRoles.sol";
import "../token/PRESS.sol";
import "../identity/PressID.sol";
import "../content/OutletRegistry.sol";
import "../content/ArticleNFT.sol";
import "./CourtModule.sol";
import "./PressCouncil.sol";

contract RoleManager is AccessRoles {
    PRESS public pressToken;
    PressID public pressId;
    OutletRegistry public outletRegistry;
    ArticleNFT public articleNFT;
    CourtModule public court;
    PressCouncil public council;

    address public owner;

    struct RoleConfig {
        uint256 cost;
        bool burns;
        bool selfAssignable;
    }

    mapping(bytes32 => RoleConfig) public roleConfigs;
    mapping(address => mapping(bytes32 => uint256)) public lockedByRole;

    event RoleConfigured(bytes32 indexed role, uint256 cost, bool burns, bool selfAssignable);
    event RolePurchased(address indexed user, bytes32 indexed role, uint256 cost, bool burned, bool locked);

    modifier onlyOwner() {
        require(msg.sender == owner, "RoleManager: not owner");
        _;
    }

    constructor(
        address superAdmin,
        address pressToken_,
        address pressId_,
        address outletRegistry_,
        address articleNFT_,
        address court_,
        address council_
    ) AccessRoles(superAdmin) {
        pressToken = PRESS(pressToken_);
        pressId = PressID(pressId_);
        outletRegistry = OutletRegistry(outletRegistry_);
        articleNFT = ArticleNFT(articleNFT_);
        court = CourtModule(court_);
        council = PressCouncil(council_);
        owner = superAdmin;

        _setRoleConfig(JOURNALIST_ROLE, 1000 ether, true, true);
        _setRoleConfig(JURY_ROLE, 500 ether, true, true);
        _setRoleConfig(MINER_ROLE, 3000 ether, true, true);
        _setRoleConfig(NEWSROOM_ADMIN_ROLE, 10000 ether, false, false);
        _setRoleConfig(COURT_ADMIN_ROLE, 25000 ether, false, false);
        _setRoleConfig(PRESS_COUNCIL_ROLE, 50000 ether, false, false);
    }

    function _setRoleConfig(bytes32 role, uint256 cost, bool burns, bool selfAssignable) internal {
        roleConfigs[role] = RoleConfig({cost: cost, burns: burns, selfAssignable: selfAssignable});
        emit RoleConfigured(role, cost, burns, selfAssignable);
    }

    function setRoleConfig(bytes32 role, uint256 cost, bool burns, bool selfAssignable)
        external
        onlyOwner
    {
        _setRoleConfig(role, cost, burns, selfAssignable);
    }

    function _handlePayment(address user, bytes32 role) internal {
        RoleConfig memory cfg = roleConfigs[role];
        require(cfg.cost > 0, "Role disabled");
        if (cfg.burns) {
            pressToken.burnFrom(user, cfg.cost);
            emit RolePurchased(user, role, cfg.cost, true, false);
        } else {
            bool ok = pressToken.transferFrom(user, address(this), cfg.cost);
            require(ok, "transfer failed");
            lockedByRole[user][role] += cfg.cost;
            emit RolePurchased(user, role, cfg.cost, false, true);
        }
    }

    function acquireJournalistRole() external {
        RoleConfig memory cfg = roleConfigs[JOURNALIST_ROLE];
        require(cfg.selfAssignable, "not self-assignable");
        _handlePayment(msg.sender, JOURNALIST_ROLE);
        _grantRole(JOURNALIST_ROLE, msg.sender);
        articleNFT.grantRole(JOURNALIST_ROLE, msg.sender);
    }

    function acquireJuryRole() external {
        RoleConfig memory cfg = roleConfigs[JURY_ROLE];
        require(cfg.selfAssignable, "not self-assignable");
        _handlePayment(msg.sender, JURY_ROLE);
        _grantRole(JURY_ROLE, msg.sender);
        court.grantRole(JURY_ROLE, msg.sender);
    }

    function governanceAssignNewsroomAdmin(address user) external onlyOwner {
        _handlePayment(user, NEWSROOM_ADMIN_ROLE);
        _grantRole(NEWSROOM_ADMIN_ROLE, user);
        outletRegistry.grantRole(NEWSROOM_ADMIN_ROLE, user);
    }

    function governanceAssignCourtAdmin(address user) external onlyOwner {
        _handlePayment(user, COURT_ADMIN_ROLE);
        _grantRole(COURT_ADMIN_ROLE, user);
        court.grantRole(COURT_ADMIN_ROLE, user);
    }

    function governanceAssignCouncilMember(address user) external onlyOwner {
        _handlePayment(user, PRESS_COUNCIL_ROLE);
        council.addCouncilMember(user);
        _grantRole(PRESS_COUNCIL_ROLE, user);
    }
}
