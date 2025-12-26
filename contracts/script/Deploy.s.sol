// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "forge-std/Script.sol";

import "../src/PressToken.sol";
import "../src/PressParameters.sol";
import "../src/CouncilMultisig.sol";
import "../src/ProposalCenter.sol";
import "../src/CouncilExecutor.sol";
import "../src/CouncilRegistry.sol";
import "../src/Court.sol";

import "../src/ArticleRegistry.sol";
import "../src/OutletRegistry.sol";
import "../src/OutletTokenFactory.sol";
import "../src/OutletMintRequests.sol";
import "../src/OutletTipPoolFactory.sol";

import "../src/OpinionRegistry.sol";
import "../src/SourceSecrecyVault.sol";
import "../src/PressLiquidityRouter.sol";
import "../src/EarningsVault.sol";
import "../src/SyndicationLicensingEngine.sol";
import "../src/DisputeBondEngine.sol";
import "../src/LegacyMigrationEngine.sol";
import "../src/TreasuryFlywheel.sol";

contract Deploy is Script {
    function _enabled(string memory key) internal view returns (bool) {
        string memory v = vm.envOr(key, string("1"));
        return keccak256(bytes(v)) != keccak256(bytes("0"));
    }

    function _w(string memory name, address a) internal {
        vm.writeFile(string.concat(vm.envString("STATE_DIR"), "/", name), string.concat(vm.toString(a), "\n"));
    }

    function run() external {
        uint256 pk = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(pk);
        

// Treasury wallet (protocol revenue)
address treasury = vm.envOr("TREASURY_WALLET", deployer);
params.setAddress(keccak256("treasury_wallet"), treasury);
        params.setUint(keccak256("tip_fee_bps"), vm.envOr("TIP_FEE_BPS", uint256(100)));
        params.setUint(keccak256("vote_fee_wei"), vm.envOr("VOTE_FEE_WEI", uint256(100000000000000)));
        params.setUint(keccak256("non_press_tip_fee_bps"), vm.envOr("NON_PRESS_TIP_FEE_BPS", uint256(200)));
        params.setUint(keccak256("tipper_bond_min_wei"), vm.envOr("TIPPER_BOND_MIN_WEI", uint256(1000000000000000000)));
params.setAddress(keccak256("article_approval"), address(articleApproval));
params.setAddress(keccak256("truth_escrow"), address(truthEscrow));
params.setAddress(keccak256("article_registry"), address(articleRegistry));
params.setAddress(keccak256("syndication_router"), address(syndication));

params.setUint(keccak256("article_vote_duration_sec"), vm.envOr("ARTICLE_VOTE_DURATION_SEC", uint256(259200)));
params.setUint(keccak256("article_vote_fee_wei"), vm.envOr("ARTICLE_VOTE_FEE_WEI", uint256(100000000000000)));
params.setUint(keccak256("article_thr_reader_yes"), vm.envOr("ARTICLE_THR_READER_YES", uint256(25)));
params.setUint(keccak256("article_thr_journalist_yes"), vm.envOr("ARTICLE_THR_JOURNALIST_YES", uint256(5)));
params.setUint(keccak256("article_thr_editor_yes"), vm.envOr("ARTICLE_THR_EDITOR_YES", uint256(2)));
params.setUint(keccak256("article_thr_outlet_yes"), vm.envOr("ARTICLE_THR_OUTLET_YES", uint256(1)));
params.setUint(keccak256("article_thr_council_yes"), vm.envOr("ARTICLE_THR_COUNCIL_YES", uint256(0)));
params.setUint(keccak256("article_max_no_ratio_bps"), vm.envOr("ARTICLE_MAX_NO_RATIO_BPS", uint256(15000)));
        params.setUint(keccak256("truth_escrow_min_wei"), vm.envOr("TRUTH_ESCROW_MIN_WEI", uint256(1000000000000000000)));
        params.setUint(keccak256("coauthor_fee_wei"), vm.envOr("COAUTHOR_FEE_WEI", uint256(100000000000000000)));
        params.setUint(keccak256("syndication_fee_bps"), vm.envOr("SYNDICATION_FEE_BPS", uint256(300)));
params.setUint(keccak256("liquidity_routing_fee_bps"), vm.envOr("LIQUIDITY_ROUTING_FEE_BPS", uint256(100)));
params.setUint(keccak256("outlet_token_std_supply_wei"), vm.envOr("OUTLET_TOKEN_STD_SUPPLY_WEI", uint256(1000000000000000000000000000))); // 1B tokens (18 decimals)
params.setUint(keccak256("outlet_token_deploy_fee_wei"), vm.envOr("OUTLET_TOKEN_DEPLOY_FEE_WEI", uint256(250000000000000000000)));
params.setUint(keccak256("outlet_token_treasury_cut_bps"), vm.envOr("OUTLET_TOKEN_TREASURY_CUT_BPS", uint256(0)));

params.setUint(keccak256("outlet_list_fee_basic_wei"), vm.envOr("OUTLET_LIST_FEE_BASIC_WEI", uint256(500000000000000000000)));
params.setUint(keccak256("outlet_list_fee_pro_wei"), vm.envOr("OUTLET_LIST_FEE_PRO_WEI", uint256(2500000000000000000000)));
params.setUint(keccak256("outlet_list_fee_inst_wei"), vm.envOr("OUTLET_LIST_FEE_INST_WEI", uint256(10000000000000000000000)));
// Outlet mint request presets (high cost; large preset triggers public vote + council multisig execution)
params.setUint(keccak256("outlet_mint_small_wei"), vm.envOr("OUTLET_MINT_SMALL_WEI", uint256(100000000000000000000000000))); // 100M
params.setUint(keccak256("outlet_mint_medium_wei"), vm.envOr("OUTLET_MINT_MEDIUM_WEI", uint256(250000000000000000000000000))); // 250M
params.setUint(keccak256("outlet_mint_large_wei"), vm.envOr("OUTLET_MINT_LARGE_WEI", uint256(500000000000000000000000000))); // 500M

params.setUint(keccak256("outlet_mint_fee_small_wei"), vm.envOr("OUTLET_MINT_FEE_SMALL_WEI", uint256(500000000000000000000))); // 500 PRESS
params.setUint(keccak256("outlet_mint_fee_medium_wei"), vm.envOr("OUTLET_MINT_FEE_MEDIUM_WEI", uint256(2500000000000000000000))); // 2500 PRESS
params.setUint(keccak256("outlet_mint_fee_large_wei"), vm.envOr("OUTLET_MINT_FEE_LARGE_WEI", uint256(10000000000000000000000))); // 10k PRESS

params.setUint(keccak256("outlet_mint_bond_small_wei"), vm.envOr("OUTLET_MINT_BOND_SMALL_WEI", uint256(250000000000000000000))); // 250 PRESS
params.setUint(keccak256("outlet_mint_bond_medium_wei"), vm.envOr("OUTLET_MINT_BOND_MEDIUM_WEI", uint256(1000000000000000000000))); // 1k PRESS
params.setUint(keccak256("outlet_mint_bond_large_wei"), vm.envOr("OUTLET_MINT_BOND_LARGE_WEI", uint256(5000000000000000000000))); // 5k PRESS

params.setUint(keccak256("outlet_mint_vote_duration_sec"), vm.envOr("OUTLET_MINT_VOTE_DURATION_SEC", uint256(604800))); // 7 days




vm.startBroadcast(pk);

        // ===== Core (required) =====
        uint256 supply = vm.envOr("PRESS_SUPPLY", uint256(1_000_000_000 ether));
        PressToken press = new PressToken(supply, deployer);

        PressParameters params = new PressParameters(deployer);
        // Council multisig owners set in installer by writing OWNER1..N env vars; fallback to deployer only.
        address owner1 = vm.envOr("COUNCIL_OWNER_1", deployer);
        address owner2 = vm.envOr("COUNCIL_OWNER_2", address(0));
        address owner3 = vm.envOr("COUNCIL_OWNER_3", address(0));
        uint256 ownerCount = 1 + (owner2 != address(0) ? 1 : 0) + (owner3 != address(0) ? 1 : 0);
        address[] memory owners = new address[](ownerCount);
        owners[0] = owner1;
        if (owner2 != address(0)) owners[1] = owner2;
        if (owner3 != address(0)) owners[ownerCount-1] = owner3;

        uint256 threshold = vm.envOr("COUNCIL_THRESHOLD", uint256(1));
        CouncilMultisig ms = new CouncilMultisig(owners, threshold);

        ProposalCenter proposals = new ProposalCenter(address(press), deployer, address(ms), address(params), address(0), address(0));
        CouncilExecutor exec = new CouncilExecutor(address(ms), address(proposals), address(params));
        CouncilRegistry councilReg = new CouncilRegistry(address(press), address(proposals), 195);
        Court court = new Court(address(press), address(councilReg), address(proposals));

        ArticleRegistry articles = new ArticleRegistry(address(press), address(params));
        OutletRegistry outlets = new OutletRegistry(address(press), deployer, address(0), address(params));
        OutletTokenFactory outletFactory = new OutletTokenFactory(address(press), address(outlets), address(proposals));
        OutletTipPoolFactory tipPools = new OutletTipPoolFactory(address(press), address(outlets), address(params));

        // Wire core addresses into parameters for cross-module reads
        params.setAddress(keccak256("press_token"), address(press));
        params.setAddress(keccak256("proposal_center"), address(proposals));
        params.setAddress(keccak256("council_multisig"), address(ms));
        params.setAddress(keccak256("council_executor"), address(exec));
        params.setAddress(keccak256("council_registry"), address(councilReg));
        params.setAddress(keccak256("press_court"), address(court));
        params.setAddress(keccak256("article_registry"), address(articles));
        params.setAddress(keccak256("outlet_registry"), address(outlets));
        params.setAddress(keccak256("outlet_token_factory"), address(outletFactory));
        params.setAddress(keccak256("outlet_tip_pools"), address(tipPools));

        // Vote fee: voting is not free
        params.setU256(keccak256("vote_fee_press"), vm.envOr("VOTE_FEE_PRESS", uint256(5 ether)));

        // ===== Optional modules (must not break when disabled) =====
        SourceSecrecyVault sourceVault;
        OpinionRegistry opinionReg;
        PressLiquidityRouter liqRouter;
        EarningsVault earningsVault;
        SyndicationLicensingEngine licensing;
        DisputeBondEngine disputeBonds;
        LegacyMigrationEngine legacy;
        TreasuryFlywheel flywheel;

        if (_enabled("MODULE_SOURCE_SECRECY_VAULT")) {
            sourceVault = new SourceSecrecyVault();
            params.setAddress(keccak256("source_secrecy_vault"), address(sourceVault));
        }
        if (_enabled("MODULE_OPINIONS")) {
            opinionReg = new OpinionRegistry(address(press), deployer, vm.envOr("OPINION_FEE_PRESS", uint256(5 ether)));
            params.setAddress(keccak256("opinion_registry"), address(opinionReg));
        }
        if (_enabled("MODULE_LIQUIDITY_ROUTING")) {
            liqRouter = new PressLiquidityRouter(address(press), deployer, vm.envOr("LIQ_ROUTER_FEE_BPS", uint256(100)));
            params.setAddress(keccak256("liquidity_router"), address(liqRouter));
        }
        if (_enabled("MODULE_EARNINGS_VAULT")) {
            earningsVault = new EarningsVault(address(press));
            params.setAddress(keccak256("earnings_vault"), address(earningsVault));
        }
        if (_enabled("MODULE_LICENSING_ENGINE")) {
            licensing = new SyndicationLicensingEngine(address(press), deployer, vm.envOr("LICENSE_PROTOCOL_CUT_BPS", uint256(500)));
            params.setAddress(keccak256("syndication_licensing_engine"), address(licensing));
        }
        if (_enabled("MODULE_DISPUTE_BONDS")) {
            disputeBonds = new DisputeBondEngine(address(press), deployer, vm.envOr("DISPUTE_MIN_BOND_PRESS", uint256(250 ether)));
            params.setAddress(keccak256("dispute_bond_engine"), address(disputeBonds));
        }
        if (_enabled("MODULE_LEGACY_MIGRATION")) {
            legacy = new LegacyMigrationEngine(address(press), deployer,
                vm.envOr("MIGRATION_FEE_PER_ITEM", uint256(2 ether)),
                vm.envOr("MIGRATION_BOND_PER_ITEM", uint256(1 ether))
            );
            params.setAddress(keccak256("legacy_migration_engine"), address(legacy));
        }
        if (_enabled("MODULE_TREASURY_FLYWHEEL")) {
            flywheel = new TreasuryFlywheel(address(press), treasury);
            params.setAddress(keccak256("treasury_flywheel"), address(flywheel));
        }

        // ===== State outputs =====
        _w("press_token_address.txt", address(press));
        _w("press_parameters_address.txt", address(params));
        _w("proposal_center_address.txt", address(proposals));
        _w("council_multisig_address.txt", address(ms));
        _w("council_executor_address.txt", address(exec));
        _w("council_registry_address.txt", address(councilReg));
        _w("press_court_address.txt", address(court));
        _w("article_registry_address.txt", address(articles));
        _w("outlet_registry_address.txt", address(outlets));
        _w("outlet_token_factory_address.txt", address(outletFactory));
        _w("outlet_tip_pools_address.txt", address(tipPools));

        if (address(sourceVault) != address(0)) _w("source_vault_address.txt", address(sourceVault));
        if (address(opinionReg) != address(0)) _w("opinion_registry_address.txt", address(opinionReg));
        if (address(liqRouter) != address(0)) _w("liquidity_router_address.txt", address(liqRouter));
        if (address(earningsVault) != address(0)) _w("earnings_vault_address.txt", address(earningsVault));
        if (address(licensing) != address(0)) _w("licensing_engine_address.txt", address(licensing));
        if (address(disputeBonds) != address(0)) _w("dispute_bonds_address.txt", address(disputeBonds));
        if (address(legacy) != address(0)) _w("legacy_migration_address.txt", address(legacy));
        if (address(flywheel) != address(0)) _w("treasury_flywheel_address.txt", address(flywheel));

        vm.stopBroadcast();
    }
}
