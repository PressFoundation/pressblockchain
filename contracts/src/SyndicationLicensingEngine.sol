// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice PRESS-priced licensing for articles (rights currency).
contract SyndicationLicensingEngine {
    address public pressToken;
    address public treasury;
    uint256 public protocolCutBps; // e.g. 500 = 5%
    uint256 public constant BPS = 10_000;

    struct License {
        bytes32 articleId;
        address buyer;
        uint256 pricePress;
        uint64  startAt;
        uint64  endAt;
        bytes32 scopeHash; // encoded scope (geo, platform, AI training flag)
    }

    event Licensed(bytes32 indexed articleId, address indexed buyer, uint256 pricePress, uint64 startAt, uint64 endAt, bytes32 scopeHash, uint256 protocolCut);

    constructor(address _pressToken, address _treasury, uint256 _cutBps) {
        pressToken = _pressToken;
        treasury = _treasury;
        protocolCutBps = _cutBps;
    }

    function setProtocolCutBps(uint256 bps) external {
        require(bps <= 1500, "CUT_TOO_HIGH"); // max 15%
        // governance later
        protocolCutBps = bps;
    }

    function license(
        bytes32 articleId,
        uint256 pricePress,
        uint64 startAt,
        uint64 endAt,
        bytes32 scopeHash,
        address payout
    ) external {
        require(articleId != bytes32(0), "BAD_ARTICLE");
        require(pricePress > 0, "BAD_PRICE");
        require(endAt == 0 || endAt > startAt, "BAD_TIME");
        require(payout != address(0), "BAD_PAYOUT");

        uint256 cut = (pricePress * protocolCutBps) / BPS;
        uint256 net = pricePress - cut;

        if (cut > 0) {
            (bool okc, ) = pressToken.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, treasury, cut));
            require(okc, "CUT_FAIL");
        }
        (bool okn, ) = pressToken.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, payout, net));
        require(okn, "PAY_FAIL");

        emit Licensed(articleId, msg.sender, pricePress, startAt, endAt, scopeHash, cut);
    }
}
