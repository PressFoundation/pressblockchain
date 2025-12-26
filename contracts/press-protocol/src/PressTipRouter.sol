// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * PressTipRouter (RR185)
 * Supports tips in PRESS and bridged assets (ETH/USDC/BTC wrappers).
 * Emits canonical events; asset adapters handle actual transfers.
 */
contract PressTipRouter {
    event TipSent(
        bytes32 indexed articleId,
        address indexed from,
        address indexed to,
        address asset,
        uint256 amount,
        uint16 protocolFeeBps,
        bytes32 ref
    );

    function emitTip(
        bytes32 articleId,
        address from,
        address to,
        address asset,
        uint256 amount,
        uint16 protocolFeeBps,
        bytes32 ref
    ) external {
        emit TipSent(articleId, from, to, asset, amount, protocolFeeBps, ref);
    }
}
