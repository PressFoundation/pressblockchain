// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

interface IPressParameters {
    function params(bytes32 key) external view returns (uint256);
}

contract ExchangeListingRegistry {
    event TokenListed(address indexed token, bytes32 indexed outletId, address indexed owner, uint8 tier, uint256 feePaid, uint256 perks);
    event TierUpgraded(address indexed token, uint8 oldTier, uint8 newTier, uint256 feePaid, uint256 perks);

    // tiers: 1 basic, 2 pro, 3 elite
    uint8 public constant TIER_BASIC = 1;
    uint8 public constant TIER_PRO   = 2;
    uint8 public constant TIER_ELITE = 3;

    // perks bitmask (expandable)
    uint256 public constant PERK_MARKETPLACE_FEATURED      = 1 << 0; // featured listing
    uint256 public constant PERK_ORACLE_BADGE              = 1 << 1; // oracle credibility badge
    uint256 public constant PERK_SYNDICATION_PRIORITY      = 1 << 2; // syndication priority
    uint256 public constant PERK_API_HIGH_RATE_LIMITS      = 1 << 3; // higher API limits
    uint256 public constant PERK_ADS_REVENUE_SHARE         = 1 << 4; // ads revenue split eligibility
    uint256 public constant PERK_ANALYTICS_SUITE           = 1 << 5; // advanced analytics
    uint256 public constant PERK_ONCHAIN_PRESSROOM         = 1 << 6; // pressroom module access
    uint256 public constant PERK_LIQUIDITY_BOOTSTRAP_TOOLS  = 1 << 7; // liquidity bootstrap tools

    address public immutable pressToken;
    address public immutable treasury;
    IPressParameters public immutable pressParams;

    struct Listing {
        bool listed;
        bytes32 outletId;
        address owner;
        uint8 tier;
        uint256 perks;
        uint256 listedAt;
        uint256 totalFeesPaid;
    }

    mapping(address => Listing) public listings; // token -> listing

    constructor(address _pressToken, address _treasury, address _pressParams) {
        pressToken = _pressToken;
        treasury = _treasury;
        pressParams = IPressParameters(_pressParams);
    }

    function _feeKey(uint8 tier) internal pure returns (bytes32) {
        if (tier == TIER_BASIC) return keccak256("listing_fee_basic");
        if (tier == TIER_PRO) return keccak256("listing_fee_pro");
        if (tier == TIER_ELITE) return keccak256("listing_fee_elite");
        revert("BAD_TIER");
    }

    function _tierPerks(uint8 tier) internal pure returns (uint256) {
        if (tier == TIER_BASIC) {
            return PERK_MARKETPLACE_FEATURED | PERK_ORACLE_BADGE;
        } else if (tier == TIER_PRO) {
            return PERK_MARKETPLACE_FEATURED | PERK_ORACLE_BADGE | PERK_SYNDICATION_PRIORITY | PERK_API_HIGH_RATE_LIMITS | PERK_ANALYTICS_SUITE;
        } else if (tier == TIER_ELITE) {
            return PERK_MARKETPLACE_FEATURED | PERK_ORACLE_BADGE | PERK_SYNDICATION_PRIORITY | PERK_API_HIGH_RATE_LIMITS |
                   PERK_ADS_REVENUE_SHARE | PERK_ANALYTICS_SUITE | PERK_ONCHAIN_PRESSROOM | PERK_LIQUIDITY_BOOTSTRAP_TOOLS;
        }
        revert("BAD_TIER");
    }

    function listToken(address token, bytes32 outletId, uint8 tier) external {
        require(token != address(0), "BAD_TOKEN");
        require(tier >= 1 && tier <= 3, "BAD_TIER");
        Listing storage L = listings[token];
        require(!L.listed, "ALREADY_LISTED");

        uint256 fee = pressParams.params(_feeKey(tier));
        if (fee > 0) require(IERC20(pressToken).transferFrom(msg.sender, treasury, fee), "FEE_FAIL");

        L.listed = true;
        L.outletId = outletId;
        L.owner = msg.sender;
        L.tier = tier;
        L.perks = _tierPerks(tier);
        L.listedAt = block.timestamp;
        L.totalFeesPaid = fee;

        emit TokenListed(token, outletId, msg.sender, tier, fee, L.perks);
    }

    function upgradeTier(address token, uint8 newTier) external {
        require(newTier >= 1 && newTier <= 3, "BAD_TIER");
        Listing storage L = listings[token];
        require(L.listed, "NOT_LISTED");
        require(msg.sender == L.owner, "NOT_OWNER");
        require(newTier > L.tier, "NOT_UPGRADE");

        uint256 oldFee = pressParams.params(_feeKey(L.tier));
        uint256 newFee = pressParams.params(_feeKey(newTier));
        uint256 delta = newFee > oldFee ? (newFee - oldFee) : 0;

        if (delta > 0) require(IERC20(pressToken).transferFrom(msg.sender, treasury, delta), "FEE_FAIL");

        uint8 oldTier = L.tier;
        L.tier = newTier;
        L.perks = _tierPerks(newTier);
        L.totalFeesPaid += delta;

        emit TierUpgraded(token, oldTier, newTier, delta, L.perks);
    }
}
