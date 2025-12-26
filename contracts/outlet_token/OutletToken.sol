// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/security/Pausable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * PRESS Outlet Token (standardized)
 * - Fixed initial supply minted to owner (deployer/outlet owner)
 * - Optional pause
 * - Simple transfer limits (maxTx / maxWallet) enforced when enabled
 * - Ownership can NOT be renounced (override)
 */
contract OutletToken is ERC20, Pausable, Ownable {
    bool public transferLimitsEnabled;
    uint256 public maxTxAmount;
    uint256 public maxWalletAmount;

    constructor(
        string memory name_,
        string memory symbol_,
        uint256 totalSupply_,
        address owner_,
        bool limitsEnabled_,
        uint256 maxTx_,
        uint256 maxWallet_
    ) ERC20(name_, symbol_) {
        _transferOwnership(owner_);
        transferLimitsEnabled = limitsEnabled_;
        maxTxAmount = maxTx_;
        maxWalletAmount = maxWallet_;
        _mint(owner_, totalSupply_);
    }

    function pause() external onlyOwner { _pause(); }
    function unpause() external onlyOwner { _unpause(); }

    function setTransferLimits(bool enabled, uint256 maxTx_, uint256 maxWallet_) external onlyOwner {
        transferLimitsEnabled = enabled;
        maxTxAmount = maxTx_;
        maxWalletAmount = maxWallet_;
    }

    function renounceOwnership() public view override onlyOwner {
        revert("RENOUNCE_DISABLED");
    }

    function _update(address from, address to, uint256 value) internal override whenNotPaused {
        if (transferLimitsEnabled) {
            if (from != address(0) && to != address(0)) {
                require(value <= maxTxAmount, "MAX_TX");
                if (to != owner()) {
                    require(balanceOf(to) + value <= maxWalletAmount, "MAX_WALLET");
                }
            }
        }
        super._update(from, to, value);
    }
}
