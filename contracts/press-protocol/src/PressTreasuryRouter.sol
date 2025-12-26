// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * PressTreasuryRouter (RR185)
 * Canonical event-based routing for all protocol fees.
 * Actual token transfers are executed by vault contracts; this router emits
 * standardized events for explorers/indexers.
 */
contract PressTreasuryRouter {
    event ProtocolFeeRouted(address indexed from, uint256 amountPress, uint16 feeBps, bytes32 ref);
    event TreasuryBurn(address indexed from, uint256 amountPress, uint16 burnBps, bytes32 ref);
    event VaultRouted(bytes32 indexed vault, address indexed to, uint256 amountPress, bytes32 ref);

    function routeProtocolFee(address from, uint256 amountPress, uint16 feeBps, uint16 burnBps, bytes32 ref) external {
        emit ProtocolFeeRouted(from, amountPress, feeBps, ref);
        if (burnBps > 0) {
            emit TreasuryBurn(from, (amountPress * burnBps) / 10000, burnBps, ref);
        }
    }

    function routeVault(bytes32 vault, address to, uint256 amountPress, bytes32 ref) external {
        emit VaultRouted(vault, to, amountPress, ref);
    }
}
