// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title PressUpgradeQueue
/// @notice On-chain queue of approved "batch upgrades" for monthly release batches.
///         Emitted events are indexer- and explorer-friendly.
contract PressUpgradeQueue {
    event BatchQueued(bytes32 indexed batchId, uint256 indexed proposalId, bytes32 indexed configKey, int256 configValue, address queuedBy, uint256 queuedAt);

    address public councilExecutor;

    modifier onlyCouncilExecutor() {
        require(msg.sender == councilExecutor, "ONLY_COUNCIL_EXECUTOR");
        _;
    }

    constructor(address _councilExecutor) {
        require(_councilExecutor != address(0), "ZERO_ADDRESS");
        councilExecutor = _councilExecutor;
    }

    function setCouncilExecutor(address _councilExecutor) external onlyCouncilExecutor {
        require(_councilExecutor != address(0), "ZERO_ADDRESS");
        councilExecutor = _councilExecutor;
    }

    function queue(bytes32 batchId, uint256 proposalId, bytes32 configKey, int256 configValue) external onlyCouncilExecutor {
        emit BatchQueued(batchId, proposalId, configKey, configValue, msg.sender, block.timestamp);
    }
}
