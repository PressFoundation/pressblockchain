// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./IERC20.sol";

/// @notice Standardized outlet token spec (fixed supply on deploy; no mint after).
/// Owner is recorded for provenance; token is non-upgradeable and anti-rug by design.
contract OutletToken is IERC20 {
    string public name;
    string public symbol;
    uint8 public immutable decimals = 18;

    uint256 public override totalSupply;
    address public immutable outletOwner;

    mapping(address => uint256) public override balanceOf;
    mapping(address => mapping(address => uint256)) public override allowance;

    constructor(string memory _name, string memory _symbol, uint256 supply, address _owner, address treasury, uint256 treasuryCut) {
        require(_owner != address(0), "owner");
        name = _name;
        symbol = _symbol;
        outletOwner = _owner;

        // supply minted on deploy; optional treasury cut for protocol alignment
        if (treasury != address(0) && treasuryCut > 0) {
            require(treasuryCut < supply, "cut");
            balanceOf[treasury] = treasuryCut;
            emit Transfer(address(0), treasury, treasuryCut);
            balanceOf[_owner] = supply - treasuryCut;
            emit Transfer(address(0), _owner, supply - treasuryCut);
        } else {
            balanceOf[_owner] = supply;
            emit Transfer(address(0), _owner, supply);
        }
        totalSupply = supply;
    }

    function transfer(address to, uint256 amount) external override returns (bool) {
        _transfer(msg.sender, to, amount);
        return true;
    }

    function approve(address spender, uint256 amount) external override returns (bool) {
        allowance[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external override returns (bool) {
        uint256 a = allowance[from][msg.sender];
        require(a >= amount, "allowance");
        allowance[from][msg.sender] = a - amount;
        _transfer(from, to, amount);
        return true;
    }

    function _transfer(address from, address to, uint256 amount) internal {
        require(to != address(0), "to");
        uint256 b = balanceOf[from];
        require(b >= amount, "bal");
        balanceOf[from] = b - amount;
        balanceOf[to] += amount;
        emit Transfer(from, to, amount);
    }
}
