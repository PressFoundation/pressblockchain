// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./PressToken.sol";
import "./PressParameters.sol";

contract TruthEscrow {
    PressToken public press;
    PressParameters public params;

    struct Escrow {
        bytes32 articleId;
        address payer;
        uint256 amount;
        bool released;
        bool slashed;
    }

    uint256 public escrowCount;
    mapping(uint256 => Escrow) public escrows;

    event EscrowOpened(uint256 indexed escrowId, bytes32 indexed articleId, address indexed payer, uint256 amount);
    event EscrowReleased(uint256 indexed escrowId, address indexed toPrimary, address toCo, uint256 primaryAmt, uint256 coAmt);
    event EscrowSlashed(uint256 indexed escrowId, address indexed treasury, uint256 amount);

    constructor(address _press, address _params){
        press = PressToken(_press);
        params = PressParameters(_params);
    }

    function open(bytes32 articleId, address payer, uint256 amount) external returns (uint256 escrowId) {
        require(amount > 0, "amount");
        escrowId = ++escrowCount;
        escrows[escrowId] = Escrow(articleId, payer, amount, false, false);
        press.transferFrom(payer, address(this), amount);
        emit EscrowOpened(escrowId, articleId, payer, amount);
    }

    function release(uint256 escrowId, address primary, address coAuthor) external {
        Escrow storage e = escrows[escrowId];
        require(!e.released && !e.slashed, "done");
        e.released = true;
        if (coAuthor != address(0)) {
            uint256 half = e.amount / 2;
            press.transfer(primary, e.amount - half);
            press.transfer(coAuthor, half);
            emit EscrowReleased(escrowId, primary, coAuthor, e.amount - half, half);
        } else {
            press.transfer(primary, e.amount);
            emit EscrowReleased(escrowId, primary, address(0), e.amount, 0);
        }
    }

    function slash(uint256 escrowId) external {
        Escrow storage e = escrows[escrowId];
        require(!e.released && !e.slashed, "done");
        e.slashed = true;
        address treasury = params.getAddress(keccak256("treasury_wallet"));
        require(treasury != address(0), "treasury unset");
        press.transfer(treasury, e.amount);
        emit EscrowSlashed(escrowId, treasury, e.amount);
    }
}
