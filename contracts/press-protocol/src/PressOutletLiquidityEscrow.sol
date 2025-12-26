// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * @title PressOutletLiquidityEscrow
 * @notice Safe escrow for outlets to optionally pre-fund liquidity actions at token deployment time.
 *         This contract does not swap or interact with external DEXs.
 *         It records ETH deposits and emits deterministic events for an off-chain router or future on-chain router.
 */
contract PressOutletLiquidityEscrow {
    struct Deposit {
        address depositor;
        address outlet;
        address token;
        uint256 amountWei;
        uint64 createdAt;
        bool claimed;
    }

    mapping(bytes32 => Deposit) public deposits;

    event LiquidityDepositCreated(bytes32 indexed depositId, address indexed depositor, address indexed outlet, address token, uint256 amountWei);

    function createDeposit(address outlet, address token) external payable returns (bytes32 depositId) {
        require(msg.value > 0, "zero");
        depositId = keccak256(abi.encode(msg.sender, outlet, token, msg.value, block.timestamp, block.chainid));
        deposits[depositId] = Deposit({
            depositor: msg.sender,
            outlet: outlet,
            token: token,
            amountWei: msg.value,
            createdAt: uint64(block.timestamp),
            claimed: false
        });
        emit LiquidityDepositCreated(depositId, msg.sender, outlet, token, msg.value);
    }
}
