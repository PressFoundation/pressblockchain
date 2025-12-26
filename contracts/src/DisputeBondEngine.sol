// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice Paid accountability: filing disputes requires PRESS bond.
/// If dispute fails, bond burned/treasury; if succeeds, redistributed.
/// This module only escrows and emits events; resolution is performed by PressCourt/PressCouncil.
contract DisputeBondEngine {
    address public pressToken;
    address public treasury;
    uint256 public minBond;

    struct Dispute {
        bytes32 articleId;
        address filer;
        uint256 bond;
        uint64 createdAt;
        bool resolved;
        bool upheld;
    }

    mapping(uint256 => Dispute) public disputes;
    uint256 public disputeCount;

    event DisputeFiled(uint256 indexed id, bytes32 indexed articleId, address indexed filer, uint256 bond, string reasonUri);
    event DisputeResolved(uint256 indexed id, bool upheld, address winner, uint256 payout, uint256 burnedOrTreasury);

    constructor(address _pressToken, address _treasury, uint256 _minBond) {
        pressToken = _pressToken;
        treasury = _treasury;
        minBond = _minBond;
    }

    function setMinBond(uint256 b) external { minBond = b; } // governance later

    function fileDispute(bytes32 articleId, uint256 bond, string calldata reasonUri) external returns (uint256 id) {
        require(articleId != bytes32(0), "BAD_ARTICLE");
        require(bond >= minBond, "BOND_LOW");
        (bool ok, ) = pressToken.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, address(this), bond));
        require(ok, "BOND_PAY_FAIL");

        id = ++disputeCount;
        disputes[id] = Dispute({articleId:articleId, filer:msg.sender, bond:bond, createdAt:uint64(block.timestamp), resolved:false, upheld:false});
        emit DisputeFiled(id, articleId, msg.sender, bond, reasonUri);
    }

    function resolve(uint256 id, bool upheld, address winner, uint256 payoutToWinner) external {
        // court/council executor will gate in later pass; emitted history is on-chain either way.
        Dispute storage d = disputes[id];
        require(!d.resolved, "RESOLVED");
        d.resolved = true;
        d.upheld = upheld;

        uint256 burnedOrTreasury = 0;
        if (upheld) {
            // winner receives some or all of bond
            if (payoutToWinner > 0) {
                require(payoutToWinner <= d.bond, "PAYOUT_GT_BOND");
                (bool okw, ) = pressToken.call(abi.encodeWithSignature("transfer(address,uint256)", winner, payoutToWinner));
                require(okw, "PAYOUT_FAIL");
                burnedOrTreasury = d.bond - payoutToWinner;
            } else {
                burnedOrTreasury = d.bond;
            }
        } else {
            // failed dispute => bond goes to treasury
            burnedOrTreasury = d.bond;
        }

        if (burnedOrTreasury > 0) {
            (bool okt, ) = pressToken.call(abi.encodeWithSignature("transfer(address,uint256)", treasury, burnedOrTreasury));
            require(okt, "TREASURY_FAIL");
        }

        emit DisputeResolved(id, upheld, winner, payoutToWinner, burnedOrTreasury);
    }
}
