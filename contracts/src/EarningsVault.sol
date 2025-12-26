// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice PRESS-native earnings vault for journalists/outlets.
/// Receives protocol-derived earnings (tips/royalties/syndication) and allows withdrawals.
/// Optional "role/bond required" policies are enforced at higher layers.
contract EarningsVault {
    address public pressToken;
    event Deposited(address indexed to, uint256 amount, string source);
    event Withdrawn(address indexed to, uint256 amount);

    mapping(address => uint256) public balance;

    constructor(address _pressToken) {
        pressToken = _pressToken;
    }

    function depositTo(address to, uint256 amount, string calldata source) external {
        require(to != address(0), "BAD_TO");
        require(amount > 0, "ZERO");
        (bool ok, ) = pressToken.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, address(this), amount));
        require(ok, "PAY_FAIL");
        balance[to] += amount;
        emit Deposited(to, amount, source);
    }

    function withdraw(uint256 amount) external {
        require(amount > 0 && balance[msg.sender] >= amount, "BAL");
        balance[msg.sender] -= amount;
        (bool ok, ) = pressToken.call(abi.encodeWithSignature("transfer(address,uint256)", msg.sender, amount));
        require(ok, "XFER_FAIL");
        emit Withdrawn(msg.sender, amount);
    }
}
