// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;
contract PressActivityProof {
    event ActivityRecorded(address indexed wallet, bytes32 indexed kind, uint256 value, bytes32 ref, uint64 ts);
    function emitActivity(address wallet, bytes32 kind, uint256 value, bytes32 ref) external {
        emit ActivityRecorded(wallet, kind, value, ref, uint64(block.timestamp));
    }
}
