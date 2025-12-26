// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IERC20T {
    function transfer(address to, uint256 value) external returns (bool);
}

contract PressTreasuryVault {
    address public owner;

    event OwnerChanged(address indexed newOwner);
    event Received(address indexed from, uint256 amount);
    event ERC20Paid(address indexed token, address indexed to, uint256 amount, string memo);
    event Paid(address indexed to, uint256 amount, string memo);

    modifier onlyOwner() { require(msg.sender == owner, "ONLY_OWNER"); _; }

    constructor(address _owner) {
        require(_owner != address(0), "ZERO_OWNER");
        owner = _owner;
    }

    function setOwner(address _o) external onlyOwner {
        require(_o != address(0), "ZERO_OWNER");
        owner = _o;
        emit OwnerChanged(_o);
    }

    receive() external payable { emit Received(msg.sender, msg.value); }

    function pay(address payable to, uint256 amount, string calldata memo) external onlyOwner {
        require(address(this).balance >= amount, "INSUFFICIENT");
        (bool ok,) = to.call{value: amount}("");
        require(ok, "PAY_FAIL");
        emit Paid(to, amount, memo);
    }

    function payERC20(address token, address to, uint256 amount, string calldata memo) external onlyOwner {
        bool ok = IERC20T(token).transfer(to, amount);
        require(ok, "TOKEN_PAY_FAIL");
        emit ERC20Paid(token, to, amount, memo);
    }
}
