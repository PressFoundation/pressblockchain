// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface ICouncilRegistry {
    function isCouncil(address who) external view returns (bool);
}

/// @notice Lightweight endorsement registry for council execution.
/// Council members endorse execution for a proposalId; count is tracked on-chain for deterministic thresholds.
contract CouncilEndorsements {
    address public councilRegistry;

    mapping(uint256 => mapping(address => bool)) public endorsed;
    mapping(uint256 => uint256) public endorsedCount;

    event Endorsed(uint256 indexed proposalId, address indexed by);

    modifier onlyCouncil() {
        require(ICouncilRegistry(councilRegistry).isCouncil(msg.sender), "COUNCIL_ONLY");
        _;
    }

    constructor(address _councilRegistry) {
        councilRegistry = _councilRegistry;
    }

    function endorse(uint256 proposalId) external onlyCouncil {
        require(!endorsed[proposalId][msg.sender], "ALREADY_ENDORSED");
        endorsed[proposalId][msg.sender] = true;
        endorsedCount[proposalId] += 1;
        emit Endorsed(proposalId, msg.sender);
    }
}
