// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./PressToken.sol";
import "./PressParameters.sol";

/// @notice High-preset outlet mint requests that auto-create an on-chain proposal-like record.
/// This is intentionally minimal and auditable. Voting + multisig execution is handled by the
/// existing proposal/council modules; this contract provides the canonical request object.
contract OutletMintRequests {
    PressToken public press;
    PressParameters public params;

    enum Preset { Small, Medium, Large }

    struct Request {
        address outletOwner;
        address token;
        Preset preset;
        uint256 mintAmount;
        uint256 feePaid;
        uint256 bondPaid;
        uint64 createdTs;
        uint64 votingEndsTs;
        bool executed;
        bool cancelled;
    }

    uint256 public requestCount;
    mapping(uint256 => Request) public requests;

    event MintRequested(uint256 indexed requestId, address indexed outletOwner, address indexed token, Preset preset, uint256 mintAmount, uint64 votingEndsTs);
    event Executed(uint256 indexed requestId);
    event Cancelled(uint256 indexed requestId);

    constructor(address _press, address _params){
        press = PressToken(_press);
        params = PressParameters(_params);
    }

    function presetMintAmount(Preset p) public view returns (uint256) {
        if (p == Preset.Small) return params.getUint(keccak256("outlet_mint_small_wei"));
        if (p == Preset.Medium) return params.getUint(keccak256("outlet_mint_medium_wei"));
        return params.getUint(keccak256("outlet_mint_large_wei"));
    }

    function presetFee(Preset p) public view returns (uint256) {
        if (p == Preset.Small) return params.getUint(keccak256("outlet_mint_fee_small_wei"));
        if (p == Preset.Medium) return params.getUint(keccak256("outlet_mint_fee_medium_wei"));
        return params.getUint(keccak256("outlet_mint_fee_large_wei"));
    }

    function presetBond(Preset p) public view returns (uint256) {
        if (p == Preset.Small) return params.getUint(keccak256("outlet_mint_bond_small_wei"));
        if (p == Preset.Medium) return params.getUint(keccak256("outlet_mint_bond_medium_wei"));
        return params.getUint(keccak256("outlet_mint_bond_large_wei"));
    }

    function requestMint(address token, Preset preset) external returns (uint256 id) {
        address treasury = params.getAddress(keccak256("treasury_wallet"));
        require(treasury != address(0), "treasury unset");

        uint256 mintAmt = presetMintAmount(preset);
        require(mintAmt > 0, "preset disabled");

        uint256 fee = presetFee(preset);
        uint256 bond = presetBond(preset);

        // Fee is revenue; bond is accountability
        if (fee > 0) press.transferFrom(msg.sender, treasury, fee);
        if (bond > 0) press.transferFrom(msg.sender, treasury, bond); // simple: treasury custody; can be escrowed later

        uint64 dur = uint64(params.getUint(keccak256("outlet_mint_vote_duration_sec")));
        if (dur == 0) dur = 7 days;

        id = ++requestCount;
        requests[id] = Request({
            outletOwner: msg.sender,
            token: token,
            preset: preset,
            mintAmount: mintAmt,
            feePaid: fee,
            bondPaid: bond,
            createdTs: uint64(block.timestamp),
            votingEndsTs: uint64(block.timestamp) + dur,
            executed: false,
            cancelled: false
        });

        emit MintRequested(id, msg.sender, token, preset, mintAmt, uint64(block.timestamp)+dur);
    }

    /// @notice Execution is expected to be called by a council multisig module once vote thresholds are met.
    /// For now, this marks executed only (actual mint is enforced by outlet token anti-rug; future: factory v2 with capped inflation).
    function markExecuted(uint256 id) external {
        Request storage r = requests[id];
        require(!r.executed && !r.cancelled, "done");
        require(block.timestamp > r.votingEndsTs, "voting active");
        r.executed = true;
        emit Executed(id);
    }

    function cancel(uint256 id) external {
        Request storage r = requests[id];
        require(msg.sender == r.outletOwner, "only owner");
        require(!r.executed && !r.cancelled, "done");
        r.cancelled = true;
        emit Cancelled(id);
    }
}
