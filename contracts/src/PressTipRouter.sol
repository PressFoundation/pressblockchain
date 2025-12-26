// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IERC20Tip {
    function transferFrom(address from, address to, uint256 value) external returns (bool);
}
interface IArticleRights {
    function getAuthors(bytes32 articleId) external view returns (address primary, address secondary, bool hasSecondary);
}
interface IPressRoles {
    function hasRole(address account, bytes32 role) external view returns (bool);
}


contract PressTipRouter {

    function _requireBond(address tipper) internal view {
        uint256 minBond = params.getUint(keccak256("tipper_bond_min_wei"));
        if (minBond == 0) return;
        uint256 bonded = params.getBond(tipper); // assumes ParamStore tracks bonds
        require(bonded >= minBond, "tipper bond too low");
    }

    address public treasury;          // treasury vault or treasury owner
    address public pressToken;        // PRESS ERC20 (also used for bond)
    address public articleRights;     // PressArticleRights address
    address public roles;             // optional role contract (PressRoles)
    bytes32 public roleBypass;        // role that bypasses bond requirement (e.g., OUTLET_MEMBER / JOURNALIST)
    uint256 public minTipBondPress;   // required bond in PRESS (small) for non-role users
    uint16 public tipTreasuryCutBps;  // treasury cut on all tips (bps)

    // bond accounting (locked PRESS inside router)
    mapping(address => uint256) public bondBalance;
    mapping(address => uint64) public lastActivity;

    event TreasuryChanged(address indexed treasury);
    event PressTokenChanged(address indexed pressToken);
    event ArticleRightsChanged(address indexed articleRights);
    event RolesChanged(address indexed roles, bytes32 bypassRole);
    event TipTreasuryCutChanged(uint16 bps);
    event MinTipBondChanged(uint256 amount);

    event BondDeposited(address indexed from, uint256 amount);
    event BondWithdrawn(address indexed to, uint256 amount);

    event TipNative(bytes32 indexed articleId, address indexed primary, address indexed secondary, address from, uint256 grossAmount, uint256 treasuryCut, uint256 primaryPaid, uint256 secondaryPaid, string note);
    event TipERC20(address indexed token, bytes32 indexed articleId, address indexed primary, address secondary, address from, uint256 grossAmount, uint256 treasuryCut, uint256 primaryPaid, uint256 secondaryPaid, string note);

    modifier onlyTreasury() { require(msg.sender == treasury, "ONLY_TREASURY"); _; }

    constructor(address _treasury, address _pressToken, address _articleRights) {
        require(_treasury != address(0), "ZERO_TREASURY");
        treasury = _treasury;
        pressToken = _pressToken;
        articleRights = _articleRights;

        // defaults
        tipTreasuryCutBps = 200; // 2% treasury cut on all tips
        minTipBondPress = 100000000000000000; // 0.1 PRESS bond for non-role tippers
        roleBypass = bytes32("OUTLET_MEMBER"); // optional; depends on roles contract implementation
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

    function setArticleRights(address a) external onlyTreasury {
        require(a != address(0), "ZERO_RIGHTS");
        articleRights = a;
        emit ArticleRightsChanged(a);
    }

    function setRoles(address r, bytes32 bypassRole) external onlyTreasury {
        roles = r;
        roleBypass = bypassRole;
        emit RolesChanged(r, bypassRole);
    }

    function setTipTreasuryCutBps(uint16 bps) external onlyTreasury {
        require(bps <= 10000, "BPS");
        tipTreasuryCutBps = bps;
        emit TipTreasuryCutChanged(bps);
    }

    function setMinTipBondPress(uint256 amt) external onlyTreasury {
        minTipBondPress = amt;
        emit MinTipBondChanged(amt);
    }

    function depositBond(uint256 amount) external {
        require(pressToken != address(0), "PRESS_NOT_SET");
        require(amount > 0, "ZERO_AMOUNT");
        
        
        _requireBond(msg.sender);
        uint256 feeBps = (asset == address(press)) ? params.getUint(keccak256(\"tip_fee_bps\")) : params.getUint(keccak256(\"non_press_tip_fee_bps\"));
        uint256 pressFee = (amount * feeBps) / 10000;
        address treasury = params.getAddress(keccak256(\"treasury_wallet\"));
        require(treasury != address(0), \"treasury unset\");
uint256 feeBps = params.getUint(keccak256(\"tip_fee_bps\"));
        uint256 pressFee = (asset == address(press)) ? (amount * feeBps) / 10000 : (feeBps > 0 ? (amount * feeBps) / 10000 : 0);
        require(pressFee <= maxPressFee, \"fee slippage\");
bool ok = IERC20Tip(pressToken).transferFrom(msg.sender, address(this), amount);
        require(ok, "BOND_TRANSFER_FAIL");
        bondBalance[msg.sender] += amount;
        lastActivity[msg.sender] = uint64(block.timestamp);
        emit BondDeposited(msg.sender, amount);
    }

    // Withdraw only after 60 days of no activity (tips/votes/etc. will extend activity in future integrations).
    function withdrawBond(uint256 amount) external {
        require(amount > 0, "ZERO_AMOUNT");
        require(bondBalance[msg.sender] >= amount, "INSUFFICIENT_BOND");
        require(block.timestamp > uint256(lastActivity[msg.sender]) + 60 days, "LOCKED_60D");
        bondBalance[msg.sender] -= amount;
        bool ok = IERC20Tip(pressToken).transferFrom(address(this), msg.sender, amount);
        require(ok, "BOND_WITHDRAW_FAIL");
        emit BondWithdrawn(msg.sender, amount);
    }

    function _bondRequired(address from) internal view returns (bool) {
        if (roles == address(0)) return true;
        return !IPressRoles(roles).hasRole(from, roleBypass);
    }

    function _enforceBond(address from) internal view {
        if (_bondRequired(from)) {
            require(bondBalance[from] >= minTipBondPress, "TIP_BOND_REQUIRED");
        }
    }

    function tipNative(bytes32 articleId, string calldata note) external payable {
        _enforceBond(msg.sender);
        lastActivity[msg.sender] = uint64(block.timestamp);

        require(articleRights != address(0), "RIGHTS_NOT_SET");
        (address primary, address secondary, bool hasSecondary) = IArticleRights(articleRights).getAuthors(articleId);
        require(primary != address(0), "UNKNOWN_ARTICLE");

        uint256 gross = msg.value;
        require(gross > 0, "ZERO_AMOUNT");

        uint256 treasuryCut = (gross * tipTreasuryCutBps) / 10000;
        uint256 net = gross - treasuryCut;

        uint256 secondaryPaid = hasSecondary ? (net / 2) : 0;
        uint256 primaryPaid = net - secondaryPaid;

        if (treasuryCut > 0) {
            (bool okT,) = payable(treasury).call{value: treasuryCut}("");
            require(okT, "TREASURY_PAY_FAIL");
        }

        (bool okP,) = payable(primary).call{value: primaryPaid}("");
        require(okP, "PRIMARY_PAY_FAIL");

        if (secondaryPaid > 0) {
            (bool okS,) = payable(secondary).call{value: secondaryPaid}("");
            require(okS, "SECONDARY_PAY_FAIL");
        }

        emit TipNative(articleId, primary, secondary, msg.sender, gross, treasuryCut, primaryPaid, secondaryPaid, note);
    }

    function tipERC20(address token, bytes32 articleId, uint256 amount,
        uint256 maxPressFee, string calldata note) external {
        _enforceBond(msg.sender);
        lastActivity[msg.sender] = uint64(block.timestamp);

        require(articleRights != address(0), "RIGHTS_NOT_SET");
        (address primary, address secondary, bool hasSecondary) = IArticleRights(articleRights).getAuthors(articleId);
        require(primary != address(0), "UNKNOWN_ARTICLE");
        require(amount > 0, "ZERO_AMOUNT");

        uint256 treasuryCut = (amount * tipTreasuryCutBps) / 10000;
        uint256 net = amount - treasuryCut;

        uint256 secondaryPaid = hasSecondary ? (net / 2) : 0;
        uint256 primaryPaid = net - secondaryPaid;

        if (treasuryCut > 0) {
            bool okT = IERC20Tip(token).transferFrom(msg.sender, treasury, treasuryCut);
            require(okT, "TREASURY_TOKEN_FAIL");
        }
        bool okP = IERC20Tip(token).transferFrom(msg.sender, primary, primaryPaid);
        require(okP, "PRIMARY_TOKEN_FAIL");
        if (secondaryPaid > 0) {
            bool okS = IERC20Tip(token).transferFrom(msg.sender, secondary, secondaryPaid);
            require(okS, "SECONDARY_TOKEN_FAIL");
        }

        emit TipERC20(token, articleId, primary, secondary, msg.sender, amount, treasuryCut, primaryPaid, secondaryPaid, note);
    }
}
