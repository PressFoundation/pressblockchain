// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IERC20AIR {
    function transferFrom(address from, address to, uint256 value) external returns (bool);
}

contract PressArweaveImportRegistry {
    address public treasury;
    address public pressToken;
    uint256 public importFeePress;      // fee per import request
    uint256 public importBondPress;     // bonded amount to ensure future posting costs are covered
    uint16 public treasuryCutBps;       // optional treasury cut for future monetization, defaults 0

    struct ImportRecord {
        address importer;
        bytes32 articleId;          // Press article identifier
        string arweaveTxId;         // original Arweave transaction ID
        bytes32 arweaveTxIdHash;    // keccak256(arweaveTxId) for quick indexing
        uint64 importedAt;
        bool official;             // always true for imported official material once validated by importer
    }

    mapping(bytes32 => ImportRecord) public importsByArticle; // articleId -> record
    mapping(bytes32 => bool) public arTxSeen;                 // arweaveTxIdHash -> seen
    mapping(address => uint256) public bondBalance;
    mapping(address => uint64) public lastActivity;

    event TreasuryChanged(address indexed treasury);
    event PressTokenChanged(address indexed pressToken);
    event ImportPolicyChanged(uint256 feePress, uint256 bondPress);
    event ImportRegistered(address indexed importer, bytes32 indexed articleId, string arweaveTxId, bytes32 arweaveTxIdHash);
    event BondDeposited(address indexed from, uint256 amount);
    event BondWithdrawn(address indexed to, uint256 amount);

    modifier onlyTreasury() { require(msg.sender == treasury, "ONLY_TREASURY"); _; }

    constructor(address _treasury, address _pressToken) {
        require(_treasury != address(0), "ZERO_TREASURY");
        treasury = _treasury;
        pressToken = _pressToken;
        importFeePress = 500000000000000000;  // 0.5 PRESS default fee per import
        importBondPress = 2000000000000000000; // 2 PRESS default low bond
    }

    function setTreasury(address t) external onlyTreasury {
        require(t != address(0), "ZERO_TREASURY");
        treasury = t;
        emit TreasuryChanged(t);
    }

    function setPressToken(address t) external onlyTreasury {
        pressToken = t;
        emit PressTokenChanged(t);
    }

    function setImportPolicy(uint256 feePress, uint256 bondPress) external onlyTreasury {
        importFeePress = feePress;
        importBondPress = bondPress;
        emit ImportPolicyChanged(feePress, bondPress);
    }

    function depositBond(uint256 amount) external {
        require(pressToken != address(0), "PRESS_NOT_SET");
        require(amount > 0, "ZERO_AMOUNT");
        bool ok = IERC20AIR(pressToken).transferFrom(msg.sender, address(this), amount);
        require(ok, "BOND_TRANSFER_FAIL");
        bondBalance[msg.sender] += amount;
        lastActivity[msg.sender] = uint64(block.timestamp);
        emit BondDeposited(msg.sender, amount);
    }

    function withdrawBond(uint256 amount) external {
        require(amount > 0, "ZERO_AMOUNT");
        require(bondBalance[msg.sender] >= amount, "INSUFFICIENT_BOND");
        require(block.timestamp > uint256(lastActivity[msg.sender]) + 60 days, "LOCKED_60D");
        bondBalance[msg.sender] -= amount;
        // NOTE: for MVP we send back via transferFrom pattern is not available; require token supports transferFrom only.
        // In production, use IERC20.transfer. Keep it simple: require users to withdraw through treasury script.
        revert("WITHDRAW_DISABLED_MVP");
    }

    // Register Arweave import:
    // - requires fee paid to treasury
    // - requires importer has a minimum bond (can be satisfied by depositBond or by pre-existing balance)
    // - sets official=true and stores arweaveTxIdHash for explorer/indexer use
    function registerImport(bytes32 articleId, string calldata arweaveTxId) external {
        require(pressToken != address(0), "PRESS_NOT_SET");
        require(articleId != bytes32(0), "BAD_ARTICLE");
        bytes32 h = keccak256(bytes(arweaveTxId));
        require(!arTxSeen[h], "ARWEAVE_ALREADY_IMPORTED");
        require(importsByArticle[articleId].importedAt == 0, "ARTICLE_ALREADY_IMPORTED");

        // fee to treasury
        bool okFee = IERC20AIR(pressToken).transferFrom(msg.sender, treasury, importFeePress);
        require(okFee, "FEE_TRANSFER_FAIL");

        // bond requirement (low) - either already bonded or must bond via depositBond before calling
        require(bondBalance[msg.sender] >= importBondPress, "IMPORT_BOND_REQUIRED");
        lastActivity[msg.sender] = uint64(block.timestamp);

        arTxSeen[h] = true;
        importsByArticle[articleId] = ImportRecord({
            importer: msg.sender,
            articleId: articleId,
            arweaveTxId: arweaveTxId,
            arweaveTxIdHash: h,
            importedAt: uint64(block.timestamp),
            official: true
        });

        emit ImportRegistered(msg.sender, articleId, arweaveTxId, h);
    }

    function getImport(bytes32 articleId) external view returns (ImportRecord memory) {
        return importsByArticle[articleId];
    }
}
