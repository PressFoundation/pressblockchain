// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IERC20AR {
    function transferFrom(address from, address to, uint256 value) external returns (bool);
}

contract PressArticleRights {
    struct Article {
        address primary;
        address secondary;
        bool hasSecondary;
        bytes32 metaHash; // e.g., keccak256(metadata JSON / IPFS / Arweave ref)
        uint64 createdAt;
    }

    address public treasury;      // treasury vault or treasury owner
    address public pressToken;    // PRESS ERC20
    uint256 public coAuthorFeePress; // flat fee to add a co-author (small)

    // optional: platform cut on sales processed through this contract (bps)
    uint16 public saleTreasuryCutBps; // 0-10000

    mapping(bytes32 => Article) public articles;

    event ArticleRegistered(bytes32 indexed articleId, address indexed primary, bytes32 metaHash);
    event CoAuthorAdded(bytes32 indexed articleId, address indexed primary, address indexed secondary, uint256 feePaid);
    event RightsSold(bytes32 indexed articleId, address indexed fromPrimary, address indexed toPrimary, uint256 grossAmount, uint256 treasuryCut, uint256 primaryPaid, uint256 secondaryPaid);
    event TreasuryChanged(address indexed treasury);
    event PressTokenChanged(address indexed pressToken);
    event CoAuthorFeeChanged(uint256 fee);
    event SaleTreasuryCutChanged(uint16 bps);

    modifier onlyTreasury() { require(msg.sender == treasury, "ONLY_TREASURY"); _; }

    constructor(address _treasury, address _pressToken) {
        require(_treasury != address(0), "ZERO_TREASURY");
        treasury = _treasury;
        pressToken = _pressToken;
        coAuthorFeePress = 100000000000000000; // 0.1 PRESS default (18 decimals)
        saleTreasuryCutBps = 200; // 2% default platform cut on rights sales (processed here)
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

    function setCoAuthorFeePress(uint256 fee) external onlyTreasury {
        coAuthorFeePress = fee;
        emit CoAuthorFeeChanged(fee);
    }

    function setSaleTreasuryCutBps(uint16 bps) external onlyTreasury {
        require(bps <= 10000, "BPS");
        saleTreasuryCutBps = bps;
        emit SaleTreasuryCutChanged(bps);
    }

    function registerArticle(bytes32 articleId, bytes32 metaHash) external {
        Article storage a = articles[articleId];
        require(a.primary == address(0), "ALREADY_REGISTERED");
        articles[articleId] = Article({
            primary: msg.sender,
            secondary: address(0),
            hasSecondary: false,
            metaHash: metaHash,
            createdAt: uint64(block.timestamp)
        });
        emit ArticleRegistered(articleId, msg.sender, metaHash);
    }

    function addCoAuthor(bytes32 articleId, address secondary) external {
        Article storage a = articles[articleId];
        require(a.primary != address(0), "NOT_REGISTERED");
        require(msg.sender == a.primary, "ONLY_PRIMARY");
        require(!a.hasSecondary, "COAUTHOR_EXISTS");
        require(secondary != address(0) && secondary != a.primary, "BAD_SECONDARY");
        // small PRESS fee to deter spam + fund treasury
        require(pressToken != address(0), "PRESS_NOT_SET");
        bool ok = IERC20AR(pressToken).transferFrom(msg.sender, treasury, coAuthorFeePress);
        require(ok, "FEE_TRANSFER_FAIL");

        a.secondary = secondary;
        a.hasSecondary = true;
        emit CoAuthorAdded(articleId, a.primary, secondary, coAuthorFeePress);
    }

    function getAuthors(bytes32 articleId) external view returns (address primary, address secondary, bool hasSecondary) {
        Article storage a = articles[articleId];
        return (a.primary, a.secondary, a.hasSecondary);
    }

    // Groundbreaking “rights sale” primitive:
    // - Primary sells control to a new primary.
    // - Proceeds auto-split 50/50 with co-author (if exists).
    // - Treasury takes a configurable bps cut.
    // - Co-author cannot block/control the sale but ALWAYS receives split.
    function sellPrimaryRights(bytes32 articleId, address newPrimary, string calldata memo) external payable {
        Article storage a = articles[articleId];
        require(a.primary != address(0), "NOT_REGISTERED");
        require(msg.sender == a.primary, "ONLY_PRIMARY");
        require(newPrimary != address(0) && newPrimary != a.primary, "BAD_NEW_PRIMARY");

        uint256 gross = msg.value;
        require(gross > 0, "ZERO_VALUE");

        uint256 treasuryCut = (gross * saleTreasuryCutBps) / 10000;
        uint256 net = gross - treasuryCut;

        uint256 primaryPaid;
        uint256 secondaryPaid;

        if (a.hasSecondary) {
            secondaryPaid = net / 2;
            primaryPaid = net - secondaryPaid;
        } else {
            primaryPaid = net;
            secondaryPaid = 0;
        }

        // pay treasury cut
        if (treasuryCut > 0) {
            (bool okT,) = payable(treasury).call{value: treasuryCut}("");
            require(okT, "TREASURY_PAY_FAIL");
        }
        // pay primary
        (bool okP,) = payable(a.primary).call{value: primaryPaid}("");
        require(okP, "PRIMARY_PAY_FAIL");
        // pay secondary
        if (secondaryPaid > 0) {
            (bool okS,) = payable(a.secondary).call{value: secondaryPaid}("");
            require(okS, "SECONDARY_PAY_FAIL");
        }

        address oldPrimary = a.primary;
        a.primary = newPrimary;

        emit RightsSold(articleId, oldPrimary, newPrimary, gross, treasuryCut, primaryPaid, secondaryPaid);

        // memo is unused on-chain beyond event indexing; included for future.
        memo;
    }
}
