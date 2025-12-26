// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;
contract PressVoteGuard {
    event VoteCast(bytes32 indexed itemId, address indexed voter, int8 direction, uint256 weight, bytes32 role, bytes32 ref, uint64 ts);
    event VoteClosed(bytes32 indexed itemId, bytes32 indexed itemType, uint64 closedAt, bytes32 reason);
    function emitVote(bytes32 itemId, address voter, int8 direction, uint256 weight, bytes32 role, bytes32 ref) external {
        emit VoteCast(itemId, voter, direction, weight, role, ref, uint64(block.timestamp));
    }
    function emitClose(bytes32 itemId, bytes32 itemType, bytes32 reason) external {
        emit VoteClosed(itemId, itemType, uint64(block.timestamp), reason);
    }
}
