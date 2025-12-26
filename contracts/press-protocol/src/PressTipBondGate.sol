// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * PressTipBondGate (RR186)
 * Non-role wallets must hold a small PRESS bond to interact with paid actions like tipping.
 * Bond can only be withdrawn after 60 days of inactivity (enforced by higher-level modules).
 * This contract emits standardized events for indexers; accounting enforced by vault modules.
 */
contract PressTipBondGate {
    event TipBondDeposited(address indexed wallet, uint256 amountPress, bytes32 ref);
    event TipBondThresholdUpdated(uint256 minBondPress, bytes32 ref);
    event TipBondWithdrawalRequested(address indexed wallet, uint256 amountPress, uint64 unlockAt, bytes32 ref);
    event TipBondWithdrawn(address indexed wallet, uint256 amountPress, bytes32 ref);

    uint256 public minBondPress;

    constructor(uint256 _minBondPress) {
        minBondPress = _minBondPress;
    }

    function emitDeposit(address wallet, uint256 amountPress, bytes32 ref) external {
        emit TipBondDeposited(wallet, amountPress, ref);
    }

    function setMinBond(uint256 _minBondPress, bytes32 ref) external {
        minBondPress = _minBondPress;
        emit TipBondThresholdUpdated(_minBondPress, ref);
    }

    function emitWithdrawalRequest(address wallet, uint256 amountPress, uint64 unlockAt, bytes32 ref) external {
        emit TipBondWithdrawalRequested(wallet, amountPress, unlockAt, ref);
    }

    function emitWithdraw(address wallet, uint256 amountPress, bytes32 ref) external {
        emit TipBondWithdrawn(wallet, amountPress, ref);
    }
}
