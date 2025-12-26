// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * @title PressOutletTokenStandard
 * @notice Standardized specs for outlet tokens (supply/decimals) to prevent abuse.
 *         Intended as a reference + enforcement helper for the deployer contracts.
 */
library PressOutletTokenStandard {
    uint8 internal constant DECIMALS = 18;

    // Hard cap per outlet token supply (default). Can be governed in future releases.
    uint256 internal constant DEFAULT_TOTAL_SUPPLY = 1_000_000_000 ether;

    // Max mint per request preset (governed elsewhere); here for shared bounds.
    uint256 internal constant MAX_MINT_PER_REQUEST = 250_000_000 ether;
}
