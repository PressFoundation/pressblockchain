// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * PressRoleDistributionRouter (RR188)
 * Canonical on-chain events for role-based pool accruals and distributions.
 * Pools apply to roles EXCLUDING: Press Council, Outlets, Special roles.
 * Distributions are even across eligible active members (enforced by vault module).
 */
contract PressRoleDistributionRouter {
    event PoolAccrued(bytes32 indexed pool, uint256 amountPress, bytes32 ref);
    event DistributionScheduled(bytes32 indexed pool, uint64 startAt, uint64 endAt, uint256 totalPress, bytes32 ref);
    event DistributionClaimed(bytes32 indexed pool, address indexed wallet, uint256 amountPress, bytes32 ref);
    event DistributionExpired(bytes32 indexed pool, uint256 unclaimedPress, bytes32 ref);

    function emitAccrual(bytes32 pool, uint256 amountPress, bytes32 ref) external {
        emit PoolAccrued(pool, amountPress, ref);
    }
    function emitSchedule(bytes32 pool, uint64 startAt, uint64 endAt, uint256 totalPress, bytes32 ref) external {
        emit DistributionScheduled(pool, startAt, endAt, totalPress, ref);
    }
    function emitClaim(bytes32 pool, address wallet, uint256 amountPress, bytes32 ref) external {
        emit DistributionClaimed(pool, wallet, amountPress, ref);
    }
    function emitExpire(bytes32 pool, uint256 unclaimedPress, bytes32 ref) external {
        emit DistributionExpired(pool, unclaimedPress, ref);
    }
}
