// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

interface ISourceRegistry {
    function sources(address a) external view returns (bool exists, bool kycIndividual, bool kycBusiness, bytes32 publicProfileHash, uint64 createdAt);
}

/**
 * @title PressSourcePool
 * @notice Minimal on-chain directory to allow explorers to enumerate sources.
 *         This is intentionally append-only for clear auditability.
 */
contract PressSourcePool {
    address public immutable registry;
    address[] public allSources;
    mapping(address => bool) public listed;

    event SourceListed(address indexed source);

    constructor(address _registry) {
        registry = _registry;
    }

    function listMySource() external {
        (bool exists,,,,) = ISourceRegistry(registry).sources(msg.sender);
        require(exists, "not_registered");
        require(!listed[msg.sender], "listed");
        listed[msg.sender] = true;
        allSources.push(msg.sender);
        emit SourceListed(msg.sender);
    }

    function count() external view returns (uint256) { return allSources.length; }

    function get(uint256 idx) external view returns (address) { return allSources[idx]; }
}
