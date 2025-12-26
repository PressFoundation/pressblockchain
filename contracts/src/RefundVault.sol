// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
}

/// @notice Holds refund reserves to eliminate "treasury must have balance" failure mode.
/// - ProposalCenter deposits refund reserve portion when proposals are created.
/// - On approval, ProposalCenter instructs RefundVault to pay proposer.
/// - Treasury can sweep surplus (governed later; deployer can be executor initially).
contract RefundVault {
    address public press;
    address public proposalCenter;
    address public treasury;

    event Paid(address indexed to, uint256 amount);
    event Swept(address indexed to, uint256 amount);
    event TreasurySet(address treasury);
    event ProposalCenterSet(address proposalCenter);

    modifier onlyProposalCenter() {
        require(msg.sender == proposalCenter, "PROPOSAL_CENTER_ONLY");
        _;
    }

    modifier onlyTreasury() {
        require(msg.sender == treasury, "TREASURY_ONLY");
        _;
    }

    constructor(address pressToken, address treasury_) {
        press = pressToken;
        treasury = treasury_;
        emit TreasurySet(treasury_);
    }

    function setProposalCenter(address pc) external onlyTreasury {
        proposalCenter = pc;
        emit ProposalCenterSet(pc);
    }

    function setTreasury(address t) external onlyTreasury {
        treasury = t;
        emit TreasurySet(t);
    }

    function pay(address to, uint256 amount) external onlyProposalCenter {
        require(IERC20(press).transfer(to, amount), "TRANSFER_FAIL");
        emit Paid(to, amount);
    }

    function sweep(address to, uint256 amount) external onlyTreasury {
        require(IERC20(press).transfer(to, amount), "TRANSFER_FAIL");
        emit Swept(to, amount);
    }
}
