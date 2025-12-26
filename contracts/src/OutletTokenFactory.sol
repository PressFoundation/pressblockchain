// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./PressToken.sol";
import "./PressParameters.sol";
import "./OutletToken.sol";
import "./PressLiquidityRouter.sol";

/// @notice Deploys standardized outlet tokens with anti-rug constraints.
/// - Fixed supply at deploy
/// - Owner is outlet wallet (provenance)
/// - Listing tiers control access + fees
/// - Protocol routing fee paid in PRESS via liquidity router
contract OutletTokenFactory {
    PressToken public press;
    PressParameters public params;
    PressLiquidityRouter public router;

    enum Tier { Basic, Pro, Institutional }

    struct OutletTokenInfo {
        address token;
        address owner;
        Tier tier;
        uint256 supply;
        uint64 createdTs;
        bool listed;
    }

    mapping(address => OutletTokenInfo) public byToken;
    mapping(address => address[]) public byOwner;

    event OutletTokenDeployed(address indexed owner, address indexed token, Tier tier, uint256 supply);
    event Listed(address indexed token, Tier tier);

    constructor(address _press, address _params, address _router){
        press = PressToken(_press);
        params = PressParameters(_params);
        router = PressLiquidityRouter(_router);
    }

    function tierFeeWei(Tier tier) public view returns (uint256) {
        if (tier == Tier.Basic) return params.getUint(keccak256("outlet_list_fee_basic_wei"));
        if (tier == Tier.Pro) return params.getUint(keccak256("outlet_list_fee_pro_wei"));
        return params.getUint(keccak256("outlet_list_fee_inst_wei"));
    }

    function deployToken(
        string calldata name,
        string calldata symbol,
        uint256 supply,
        Tier tier
    ) external returns (address token) {
        require(supply > 0, "supply");

        // Standardized specs enforcement
        uint256 stdSupply = params.getUint(keccak256("outlet_token_std_supply_wei"));
        if (stdSupply != 0) require(supply == stdSupply, "must match std supply");

        address treasury = params.getAddress(keccak256("treasury_wallet"));
        require(treasury != address(0), "treasury unset");

        // Deployment fee paid in PRESS, 50% treasury, 50% retained by token owner as initial "liquidity seed" (held in PRESS)
        uint256 deployFee = params.getUint(keccak256("outlet_token_deploy_fee_wei"));
        if (deployFee == 0) deployFee = 250000000000000000000; // 250 PRESS default

        uint256 toTreasury = deployFee / 2;
        uint256 toOwner = deployFee - toTreasury;
        press.transferFrom(msg.sender, treasury, toTreasury);
        press.transferFrom(msg.sender, msg.sender, toOwner); // intentionally leaves with owner; UX later can guide LP add

        // protocol liquidity routing fee
        router.charge(keccak256("OUTLET_TOKEN_DEPLOY"), msg.sender, deployFee);

        // optional treasury cut of token supply (for protocol alignment)
        uint256 cutBps = params.getUint(keccak256("outlet_token_treasury_cut_bps"));
        uint256 cut = (supply * cutBps) / 10000;

        token = address(new OutletToken(name, symbol, supply, msg.sender, treasury, cut));
        byToken[token] = OutletTokenInfo(token, msg.sender, tier, supply, uint64(block.timestamp), false);
        byOwner[msg.sender].push(token);

        emit OutletTokenDeployed(msg.sender, token, tier, supply);
    }

    function listToken(address token) external {
        OutletTokenInfo storage info = byToken[token];
        require(info.token != address(0), "unknown");
        require(info.owner == msg.sender, "only owner");
        require(!info.listed, "listed");

        uint256 fee = tierFeeWei(info.tier);
        if (fee == 0) {
            // defaults: increasingly high with access/value expectations handled by exchange UI
            if (info.tier == Tier.Basic) fee = 500000000000000000000; // 500 PRESS
            else if (info.tier == Tier.Pro) fee = 2500000000000000000000; // 2500 PRESS
            else fee = 10000000000000000000000; // 10,000 PRESS
        }

        address treasury = params.getAddress(keccak256("treasury_wallet"));
        press.transferFrom(msg.sender, treasury, fee);
        router.charge(keccak256("OUTLET_TOKEN_LIST"), msg.sender, fee);

        info.listed = true;
        emit Listed(token, info.tier);
    }

    function tokensOf(address owner) external view returns (address[] memory) {
        return byOwner[owner];
    }
}
