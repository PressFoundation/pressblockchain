// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

interface IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

/// @notice Enforces protocol fee collection in PRESS for key actions (publish/vote/co-author/import/listing).
/// This contract is intentionally simple: it collects to treasury and emits events that explorers/indexers can read.
contract PressFeeRouter {
    event FeePaid(bytes32 indexed context, address indexed payer, uint256 amount, bytes32 indexed ref);
    address public immutable pressToken;
    address public treasury;
    uint16 public protocolFeeBps; // 0-10000

    mapping(bytes32 => uint256) public fixedFee; // context => fee in PRESS wei
    // contexts are predefined hashes (e.g. keccak256("VOTE"), keccak256("PUBLISH"))

    constructor(address _pressToken, address _treasury, uint16 _bps) {
        pressToken=_pressToken;
        treasury=_treasury;
        protocolFeeBps=_bps;
    }

    function setTreasury(address t) external {
        // Governance-wired later; installer owns this contract initially in devnets.
        treasury=t;
    }

    function setProtocolFeeBps(uint16 bps) external {
        require(bps<=10000,"bps");
        protocolFeeBps=bps;
    }

    function setFixedFee(bytes32 ctx, uint256 feeWei) external {
        fixedFee[ctx]=feeWei;
    }

    function pay(bytes32 ctx, uint256 amountWei, bytes32 ref) external {
        uint256 fee = fixedFee[ctx];
        require(amountWei>=fee, "fee");
        require(IERC20(pressToken).transferFrom(msg.sender, treasury, amountWei), "transferFrom");
        emit FeePaid(ctx, msg.sender, amountWei, ref);
    }
}
