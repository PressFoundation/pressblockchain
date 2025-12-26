// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IOutletRegistryMembers {
    function membersOf(bytes32 outletId) external view returns (address[] memory);
    function outletManagers(bytes32 outletId) external view returns (address);
    function outlets(bytes32 outletId) external view returns (bool exists, address manager, string memory name, string memory domain, uint256 createdAt);
}

interface IERC20Pool {
    function transfer(address to, uint256 value) external returns (bool);
    function transferFrom(address from, address to, uint256 value) external returns (bool);
}

/// @notice Per-outlet pool that accumulates tips and can distribute evenly to outlet members.
/// @dev Gas scales with member count; keep outlet member counts reasonable or distribute in batches.
contract OutletTipPool {
    bytes32 public outletId;
    address public outletRegistry;
    address public treasury;
    uint256 public distributorFeeBps; // paid from pool at distribution time (e.g. 50 = 0.50%)
    uint256 public constant BPS = 10_000;

    event TipNative(bytes32 indexed outletId, address indexed from, uint256 amount, uint256 treasuryCut, string note);
    event TipERC20(bytes32 indexed outletId, address indexed token, address indexed from, uint256 amount, uint256 treasuryCut, string note);
    event Distributed(bytes32 indexed outletId, address indexed token, uint256 members, uint256 total, uint256 perMember, uint256 distributorFee);

    modifier onlyManager() {
        (, address manager, , , ) = IOutletRegistryMembers(outletRegistry).outlets(outletId);
        require(msg.sender == manager, "MANAGER_ONLY");
        _;
    }

    constructor(bytes32 _outletId, address _registry, address _treasury, uint256 _distributorFeeBps) {
        outletId = _outletId;
        outletRegistry = _registry;
        treasury = _treasury;
        distributorFeeBps = _distributorFeeBps;
    }

    receive() external payable {}

    function setDistributorFeeBps(uint256 bps) external onlyManager {
        require(bps <= 200, "FEE_TOO_HIGH"); // max 2%
        distributorFeeBps = bps;
    }

    function tipNative(uint256 treasuryCutBps, string calldata note) external payable {
        require(msg.value > 0, "ZERO");
        uint256 tcut = (msg.value * treasuryCutBps) / BPS;
        if (tcut > 0) {
            (bool ok,) = treasury.call{value:tcut}("");
            require(ok, "TREASURY_FAIL");
        }
        emit TipNative(outletId, msg.sender, msg.value, tcut, note);
    }

    function tipERC20(address token, uint256 amount, uint256 treasuryCutBps, string calldata note) external {
        require(amount > 0, "ZERO");
        uint256 tcut = (amount * treasuryCutBps) / BPS;
        if (tcut > 0) {
            require(IERC20Pool(token).transferFrom(msg.sender, treasury, tcut), "TREASURY_TOK_FAIL");
        }
        require(IERC20Pool(token).transferFrom(msg.sender, address(this), amount - tcut), "POOL_TOK_FAIL");
        emit TipERC20(outletId, token, msg.sender, amount, tcut, note);
    }

    /// @notice Distribute native balance evenly to members.
    function distributeNative(uint256 maxMembers) external {
        address[] memory mem = IOutletRegistryMembers(outletRegistry).membersOf(outletId);
        require(mem.length > 0, "NO_MEMBERS");
        require(mem.length <= maxMembers, "TOO_MANY_MEMBERS");
        uint256 bal = address(this).balance;
        require(bal > 0, "NO_FUNDS");

        uint256 fee = (bal * distributorFeeBps) / BPS;
        uint256 net = bal - fee;
        uint256 per = net / mem.length;
        require(per > 0, "DUST");

        if (fee > 0) {
            (bool okf,) = payable(msg.sender).call{value:fee}("");
            require(okf, "FEE_PAY_FAIL");
        }
        for (uint256 i=0;i<mem.length;i++) {
            (bool ok,) = payable(mem[i]).call{value:per}("");
            require(ok, "PAY_FAIL");
        }
        emit Distributed(outletId, address(0), mem.length, bal, per, fee);
    }

    /// @notice Distribute ERC20 balance evenly to members.
    function distributeERC20(address token, uint256 maxMembers) external {
        address[] memory mem = IOutletRegistryMembers(outletRegistry).membersOf(outletId);
        require(mem.length > 0, "NO_MEMBERS");
        require(mem.length <= maxMembers, "TOO_MANY_MEMBERS");

        // balance via ERC20 transfer() return doesn't provide balanceOf; assume token supports standard balanceOf?
        // Keep minimal: require caller passes token that has balanceOf; if not, distribution will revert in next pass.
        (bool ok, bytes memory data) = token.staticcall(abi.encodeWithSignature("balanceOf(address)", address(this)));
        require(ok && data.length >= 32, "BALANCEOF_UNSUPPORTED");
        uint256 bal = abi.decode(data, (uint256));
        require(bal > 0, "NO_FUNDS");

        uint256 fee = (bal * distributorFeeBps) / BPS;
        uint256 net = bal - fee;
        uint256 per = net / mem.length;
        require(per > 0, "DUST");

        if (fee > 0) {
            require(IERC20Pool(token).transfer(msg.sender, fee), "FEE_TOK_FAIL");
        }
        for (uint256 i=0;i<mem.length;i++) {
            require(IERC20Pool(token).transfer(mem[i], per), "PAY_TOK_FAIL");
        }
        emit Distributed(outletId, token, mem.length, bal, per, fee);
    }
}

contract OutletTipPoolFactory {
    address public outletRegistry;
    address public treasury;
    uint256 public defaultDistributorFeeBps = 50; // 0.50%

    mapping(bytes32 => address) public poolOf;

    event OutletPoolCreated(bytes32 indexed outletId, address pool);

    constructor(address _registry, address _treasury) {
        outletRegistry = _registry;
        treasury = _treasury;
    }

    function setDefaultDistributorFeeBps(uint256 bps) external {
        require(bps <= 200, "FEE_TOO_HIGH");
        // governance in later pass
        defaultDistributorFeeBps = bps;
    }

    function createPool(bytes32 outletId) external returns (address) {
        require(poolOf[outletId] == address(0), "POOL_EXISTS");
        // allow manager or anyone; manager can adjust fee later
        OutletTipPool pool = new OutletTipPool(outletId, outletRegistry, treasury, defaultDistributorFeeBps);
        poolOf[outletId] = address(pool);
        emit OutletPoolCreated(outletId, address(pool));
        return address(pool);
    }
}
