// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./PressToken.sol";
import "./PressParameters.sol";

/// @notice Protocol liquidity rails for outlet economies.
/// Enforces routing fees (0.5-2%) paid in PRESS on token listing + key flows.
contract PressLiquidityRouter {
    PressToken public press;
    PressParameters public params;

    event Routed(address indexed payer, uint256 feeWei, bytes32 indexed reason);

    constructor(address _press, address _params){
        press = PressToken(_press);
        params = PressParameters(_params);
    }

    function charge(bytes32 reason, address payer, uint256 baseWei) external returns (uint256 feeWei) {
        address treasury = params.getAddress(keccak256("treasury_wallet"));
        require(treasury != address(0), "treasury unset");

        uint256 bps = params.getUint(keccak256("liquidity_routing_fee_bps"));
        if (bps == 0) bps = 100; // 1% default
        // allow reason overrides via params if desired later
        feeWei = (baseWei * bps) / 10000;
        if (feeWei > 0) {
            press.transferFrom(payer, treasury, feeWei);
            emit Routed(payer, feeWei, reason);
        }
    }
}
