// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/// @notice Minimal treasury receiver for protocol fees. Ownership is externalized via governance later.
contract PressTreasury {
    event Received(address indexed from, uint256 amount);
    receive() external payable { emit Received(msg.sender, msg.value); }
}
