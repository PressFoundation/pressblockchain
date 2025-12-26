// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "forge-std/Script.sol";
import "../src/PressTreasury.sol";
import "../src/PressFeeRouter.sol";
import "../src/PressGovernanceSignals.sol";

contract DeployPressProtocol is Script {
    function run() external {
        address pressToken = vm.envAddress("PRESS_TOKEN");
        address treasuryOwner = vm.envAddress("TREASURY_OWNER");
        uint16 bps = uint16(vm.envUint("PROTOCOL_FEE_BPS"));

        vm.startBroadcast();

        PressTreasury treasury = new PressTreasury();
        PressFeeRouter router = new PressFeeRouter(pressToken, address(treasury), bps);
        PressGovernanceSignals signals = new PressGovernanceSignals();

        // set default fixed fees (all in wei, 18 decimals)
        router.setFixedFee(keccak256("VOTE"), vm.envUint("FEE_VOTE_WEI"));
        router.setFixedFee(keccak256("PUBLISH"), vm.envUint("FEE_PUBLISH_WEI"));
        router.setFixedFee(keccak256("COAUTHOR"), vm.envUint("FEE_COAUTHOR_WEI"));
        router.setFixedFee(keccak256("IMPORT_ARWEAVE"), vm.envUint("FEE_IMPORT_WEI"));

        vm.stopBroadcast();

        console2.log("TREASURY", address(treasury));
        console2.log("FEE_ROUTER", address(router));
        console2.log("GOV_SIGNALS", address(signals));
    }
}
