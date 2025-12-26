// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice Canonical on-chain parameter registry for Press Blockchain governance.
/// Keys are bytes32 identifiers; values are uint256. Only the CouncilExecutor can update.
contract PressParameters {
    address public councilExecutor;

    mapping(bytes32 => uint256) public params;

    event ParamSet(bytes32 indexed key, uint256 value, address indexed by);

    modifier onlyExecutor() {
        require(msg.sender == councilExecutor, "EXECUTOR_ONLY");
        _;
    }

    constructor(address executor) {
setUint(keccak256("non_press_tip_fee_bps"), 200); // 2% routing fee for non-PRESS tips
setUint(keccak256("tipper_bond_min_wei"), 1000000000000000000); // 1 PRESS bond

setUint(keccak256("treasury_fee_bps"), 100); // 1%
setUint(keccak256("tip_fee_bps"), 100); // 1%
setUint(keccak256("vote_fee_wei"), 100000000000000); // 0.0001

        councilExecutor = executor;
    }

    function setExecutor(address executor) external onlyExecutor {
        councilExecutor = executor;
    }

    function set(bytes32 key, uint256 value) external onlyExecutor {
        params[key] = value;
        emit ParamSet(key, value, msg.sender);
    }
}
