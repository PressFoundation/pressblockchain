// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./PressToken.sol";
import "./PressParameters.sol";

contract SyndicationRouter {
    PressToken public press;
    PressParameters public params;

    struct License {
        bytes32 articleId;
        address buyer;
        uint64 startTs;
        uint64 endTs;
        uint256 price;
        bytes32 scopeHash;
        bool active;
    }

    uint256 public licenseCount;
    mapping(uint256 => License) public licenses;

    event Licensed(uint256 indexed licenseId, bytes32 indexed articleId, address indexed buyer, uint64 startTs, uint64 endTs, uint256 price);

    constructor(address _press, address _params){
        press = PressToken(_press);
        params = PressParameters(_params);
    }

    function license(bytes32 articleId, uint64 durationSec, uint256 price, bytes32 scopeHash) external returns (uint256 id) {
        require(durationSec > 0, "duration");
        require(price > 0, "price");
        address treasury = params.getAddress(keccak256("treasury_wallet"));
        require(treasury != address(0), "treasury unset");

        uint256 feeBps = params.getUint(keccak256("syndication_fee_bps"));
        if (feeBps == 0) feeBps = 300;
        uint256 fee = (price * feeBps) / 10000;

        press.transferFrom(msg.sender, treasury, fee);
        press.transferFrom(msg.sender, address(this), price - fee);

        id = ++licenseCount;
        uint64 start = uint64(block.timestamp);
        licenses[id] = License(articleId, msg.sender, start, start + durationSec, price, scopeHash, true);

        emit Licensed(id, articleId, msg.sender, start, start + durationSec, price);
    }

    function claimRevenue(uint256 licenseId, address primary, address coAuthor) external {
        License storage L = licenses[licenseId];
        require(L.active, "inactive");
        L.active = false;

        uint256 feeBps = params.getUint(keccak256("syndication_fee_bps"));
        if (feeBps == 0) feeBps = 300;
        uint256 net = L.price - ((L.price * feeBps) / 10000);
        if (coAuthor != address(0)) {
            uint256 half = net/2;
            press.transfer(primary, net - half);
            press.transfer(coAuthor, half);
        } else {
            press.transfer(primary, net);
        }
    }
}
