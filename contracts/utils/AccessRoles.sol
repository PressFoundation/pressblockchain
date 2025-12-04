// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/AccessControl.sol";

contract AccessRoles is AccessControl {
    bytes32 public constant SUPER_ADMIN_ROLE    = keccak256("SUPER_ADMIN_ROLE");
    bytes32 public constant NEWSROOM_ADMIN_ROLE = keccak256("NEWSROOM_ADMIN_ROLE");
    bytes32 public constant JOURNALIST_ROLE     = keccak256("JOURNALIST_ROLE");
    bytes32 public constant COURT_ADMIN_ROLE    = keccak256("COURT_ADMIN_ROLE");
    bytes32 public constant JURY_ROLE           = keccak256("JURY_ROLE");
    bytes32 public constant MINER_ROLE          = keccak256("MINER_ROLE");
    bytes32 public constant PRESS_COUNCIL_ROLE  = keccak256("PRESS_COUNCIL_ROLE");
    bytes32 public constant TRUST_ENGINE_ROLE   = keccak256("TRUST_ENGINE_ROLE");

    constructor(address superAdmin) {
        _grantRole(DEFAULT_ADMIN_ROLE, superAdmin);
        _grantRole(SUPER_ADMIN_ROLE, superAdmin);
    }
}
