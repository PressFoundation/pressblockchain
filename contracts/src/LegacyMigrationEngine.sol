// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice One-Click Legacy Migration: import archives with per-item fees + bond.
/// Each import produces a TXID-like on-chain event (importId).
contract LegacyMigrationEngine {
    address public pressToken;
    address public treasury;
    uint256 public feePerItem; // per article/content unit
    uint256 public bondPerItem; // minimum bond coverage per imported item

    struct ImportJob {
        address requester;
        uint256 itemCount;
        uint256 feePaid;
        uint256 bondLocked;
        uint64 createdAt;
        string manifestUri; // list of content pointers/hashes
        bool completed;
    }

    mapping(uint256 => ImportJob) public jobs;
    uint256 public jobCount;

    event ImportRequested(uint256 indexed importId, address indexed requester, uint256 itemCount, uint256 feePaid, uint256 bondLocked, string manifestUri);
    event ImportCompleted(uint256 indexed importId, bytes32 rootHash, uint256 anchoredCount);

    constructor(address _pressToken, address _treasury, uint256 _feePerItem, uint256 _bondPerItem) {
        pressToken = _pressToken;
        treasury = _treasury;
        feePerItem = _feePerItem;
        bondPerItem = _bondPerItem;
    }

    function setPricing(uint256 _feePerItem, uint256 _bondPerItem) external { feePerItem=_feePerItem; bondPerItem=_bondPerItem; } // governance later

    function requestImport(uint256 itemCount, string calldata manifestUri) external returns (uint256 importId) {
        require(itemCount > 0, "COUNT");
        require(bytes(manifestUri).length > 0, "URI");
        uint256 fee = feePerItem * itemCount;
        uint256 bond = bondPerItem * itemCount;

        if (fee > 0) {
            (bool okf, ) = pressToken.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, treasury, fee));
            require(okf, "FEE_FAIL");
        }
        if (bond > 0) {
            (bool okb, ) = pressToken.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, address(this), bond));
            require(okb, "BOND_FAIL");
        }

        importId = ++jobCount;
        jobs[importId] = ImportJob({requester:msg.sender, itemCount:itemCount, feePaid:fee, bondLocked:bond, createdAt:uint64(block.timestamp), manifestUri:manifestUri, completed:false});
        emit ImportRequested(importId, msg.sender, itemCount, fee, bond, manifestUri);
    }

    function markCompleted(uint256 importId, bytes32 rootHash, uint256 anchoredCount) external {
        // indexer/oracle will gate later; for MVP this is a log anchor
        ImportJob storage j = jobs[importId];
        require(!j.completed, "DONE");
        j.completed = true;
        emit ImportCompleted(importId, rootHash, anchoredCount);
    }
}
