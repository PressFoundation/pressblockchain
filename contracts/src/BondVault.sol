// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

interface IERC20 {
    function transferFrom(address from, address to, uint256 v) external returns (bool);
    function transfer(address to, uint256 v) external returns (bool);
    function balanceOf(address a) external view returns (uint256);
}

contract BondVault {
    IERC20 public press;
    address public governance;
    uint64 public constant WITHDRAW_COOLDOWN_SECS = 60 days;

    // role keys: keccak256("OUTLET_BOND"), keccak256("COUNCIL_BOND")
    mapping(address => mapping(bytes32 => uint256)) public bonded;
    mapping(address => uint64) public lastActivity;

    event GovernanceSet(address indexed governance);
    event BondDeposited(address indexed account, bytes32 indexed role, uint256 amount);
    event ActivityTouched(address indexed account, uint64 ts);
    event BondWithdrawn(address indexed account, bytes32 indexed role, uint256 amount);

    modifier onlyGov() {
        require(msg.sender == governance, "GOV_ONLY");
        _;
    }

    constructor(address pressToken, address gov) {
        press = IERC20(pressToken);
        governance = gov;
        emit GovernanceSet(gov);
    }

    function setGovernance(address gov) external onlyGov {
        governance = gov;
        emit GovernanceSet(gov);
    }

    function touchActivity(address account) external onlyGov {
        lastActivity[account] = uint64(block.timestamp);
        emit ActivityTouched(account, uint64(block.timestamp));
    }

    function deposit(bytes32 role, uint256 amount) external {
        require(amount > 0, "AMOUNT");
        require(press.transferFrom(msg.sender, address(this), amount), "TRANSFER");
        bonded[msg.sender][role] += amount;
        lastActivity[msg.sender] = uint64(block.timestamp);
        emit BondDeposited(msg.sender, role, amount);
    }

    function canWithdraw(address account) public view returns (bool) {
        uint64 t = lastActivity[account];
        if (t == 0) return true;
        return block.timestamp >= uint256(t) + WITHDRAW_COOLDOWN_SECS;
    }

    function withdraw(bytes32 role, uint256 amount) external {
        require(canWithdraw(msg.sender), "COOLDOWN");
        uint256 b = bonded[msg.sender][role];
        require(b >= amount, "BOND");
        bonded[msg.sender][role] = b - amount;
        require(press.transfer(msg.sender, amount), "TRANSFER");
        emit BondWithdrawn(msg.sender, role, amount);
    }
}
