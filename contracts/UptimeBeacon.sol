// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title UptimeBeacon - minimal on-chain heartbeat for Press Blockchain services
/// @notice Emits Heartbeat events; no admin control; intended for transparency/monitoring.
contract UptimeBeacon {
    event Heartbeat(bytes32 indexed service, address indexed caller, uint64 ts, uint8 status, bytes32 extra);

    function heartbeat(bytes32 service, uint8 status, bytes32 extra) external {
        emit Heartbeat(service, msg.sender, uint64(block.timestamp), status, extra);
    }
}
