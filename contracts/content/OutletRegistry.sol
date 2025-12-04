// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../utils/AccessRoles.sol";

contract OutletRegistry is AccessRoles {
    struct Outlet {
        uint256 id;
        address owner;
        string name;
        string countryCode;
        string metadataURI;
        bool active;
    }

    uint256 public nextOutletId;
    mapping(uint256 => Outlet) private _outlets;

    event OutletRegistered(
        uint256 indexed id,
        address indexed owner,
        string name,
        string countryCode,
        string metadataURI
    );
    event OutletUpdated(
        uint256 indexed id,
        string name,
        string countryCode,
        string metadataURI,
        bool active
    );

    constructor(address superAdmin) AccessRoles(superAdmin) {}

    function registerOutlet(
        string calldata name,
        string calldata countryCode,
        string calldata metadataURI
    ) external onlyRole(NEWSROOM_ADMIN_ROLE) returns (uint256) {
        uint256 id = ++nextOutletId;
        _outlets[id] = Outlet({
            id: id,
            owner: msg.sender,
            name: name,
            countryCode: countryCode,
            metadataURI: metadataURI,
            active: true
        });
        emit OutletRegistered(id, msg.sender, name, countryCode, metadataURI);
        return id;
    }

    function updateOutlet(
        uint256 id,
        string calldata name,
        string calldata countryCode,
        string calldata metadataURI,
        bool active
    ) external {
        Outlet storage o = _outlets[id];
        require(o.owner == msg.sender || hasRole(SUPER_ADMIN_ROLE, msg.sender), "Not authorized");
        o.name = name;
        o.countryCode = countryCode;
        o.metadataURI = metadataURI;
        o.active = active;
        emit OutletUpdated(id, name, countryCode, metadataURI, active);
    }

    function getOutlet(uint256 id) external view returns (Outlet memory) {
        return _outlets[id];
    }
}
