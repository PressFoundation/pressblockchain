// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice Treasury flywheel hooks: all fees route to treasury; treasury can buy/burn PRESS using on-chain policies.
/// This module is a governance-controlled orchestrator (no direct DEX logic here in MVP).
contract TreasuryFlywheel {
    address public pressToken;
    address public treasury;
    uint256 public lastActionAt;

    event PolicySignal(string action, uint256 amount, string note);

    constructor(address _pressToken, address _treasury) {
        pressToken = _pressToken;
        treasury = _treasury;
    }

    function signalBuyBack(uint256 amountPress, string calldata note) external {
        // governance later
        lastActionAt = block.timestamp;
        emit PolicySignal("BUYBACK", amountPress, note);
    }

    function signalBurn(uint256 amountPress, string calldata note) external {
        // governance later
        lastActionAt = block.timestamp;
        emit PolicySignal("BURN", amountPress, note);
    }

    function signalGrant(uint256 amountPress, string calldata note) external {
        // governance later
        lastActionAt = block.timestamp;
        emit PolicySignal("GRANT", amountPress, note);
    }
}
