// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

interface IERC20Burn {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function burn(uint256 amount) external;
}

/**
 * @title PressBurnController
 * @notice Yearly burn coordinator.
 *         Burns only from a dedicated "burn vault" funded by protocol fees.
 *         Author/Source earnings never flow here.
 *
 * Safety:
 * - Burn can only execute once per yearId.
 * - Emits a deterministic event for explorers.
 */
contract PressBurnController {
    address public immutable pressToken;
    address public burnVault;
    mapping(uint256 => bool) public burnedYear;

    event BurnVaultSet(address indexed vault);
    event YearlyBurnExecuted(uint256 indexed yearId, uint256 amount);

    constructor(address _pressToken, address _burnVault) {
        pressToken=_pressToken;
        burnVault=_burnVault;
        emit BurnVaultSet(_burnVault);
    }

    function setBurnVault(address v) external {
        burnVault=v;
        emit BurnVaultSet(v);
    }

    function executeYearlyBurn(uint256 yearId, uint256 amount) external {
        require(!burnedYear[yearId], "already_burned");
        burnedYear[yearId]=true;
        // Pull from burn vault via allowance to keep custody separate
        require(IERC20Burn(pressToken).transferFrom(burnVault, address(this), amount), "pull_failed");
        IERC20Burn(pressToken).burn(amount);
        emit YearlyBurnExecuted(yearId, amount);
    }
}
