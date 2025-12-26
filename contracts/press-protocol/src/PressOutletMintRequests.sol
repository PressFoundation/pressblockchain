// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

interface IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

library PressErrors {
    error Unauthorized();
    error InvalidPreset();
    error RequestNotFound();
    error AlreadyFinalized();
    error Cooldown();
}

/**
 * @title PressOutletMintRequests
 * @notice Outlet can request additional mint allocations via preset tiers.
 * - Small presets: immediate if paid
 * - Large preset: auto-creates an on-chain "proposal intent" event for public vote
 *
 * This contract intentionally emits explorer-friendly events so Blockscout/custom explorers
 * can render the entire workflow even before a full governance module is attached.
 */
contract PressOutletMintRequests {
    using PressErrors for *;

    enum Preset { NONE, SMALL, MEDIUM, LARGE }

    struct Request {
        address outlet;
        address token;
        Preset preset;
        uint256 amount;
        uint256 feePress;
        uint256 bondPress;
        uint64 createdAt;
        bool finalized;
        bool approved; // for LARGE preset, set via governance executor in later module
    }

    IERC20 public immutable pressToken;
    address public treasury;
    uint256 public requestCount;

    // Fee schedule in PRESS (can be governed later)
    uint256 public feeSmall = 25_000 ether;
    uint256 public feeMedium = 75_000 ether;
    uint256 public feeLarge = 200_000 ether;

    // Bond schedule in PRESS (covers activity + anti-spam)
    uint256 public bondSmall = 50_000 ether;
    uint256 public bondMedium = 150_000 ether;
    uint256 public bondLarge = 500_000 ether;

    // Mint amounts (token units, 18 decimals assumed)
    uint256 public mintSmall = 10_000_000 ether;
    uint256 public mintMedium = 50_000_000 ether;
    uint256 public mintLarge = 200_000_000 ether;

    // Cooldown to prevent spamming
    mapping(address => uint64) public lastRequestAt;

    event OutletMintRequested(
        uint256 indexed requestId,
        address indexed outlet,
        address indexed token,
        Preset preset,
        uint256 amount,
        uint256 feePress,
        uint256 bondPress
    );

    // For LARGE preset: explorers and indexers treat this as an auto-proposal creation intent.
    event OutletMintProposalIntent(
        uint256 indexed requestId,
        address indexed outlet,
        address indexed token,
        uint256 amount,
        bytes32 proposalHash
    );

    event OutletMintFinalized(uint256 indexed requestId, bool approved);

    mapping(uint256 => Request) public requests;

    constructor(address _pressToken, address _treasury) {
        pressToken = IERC20(_pressToken);
        treasury = _treasury;
    }

    function setTreasury(address t) external {
        // In a governed deployment this would be restricted; left open for sovereign bootstrap.
        treasury = t;
    }

    function requestMint(address outletToken, Preset preset) external returns (uint256 requestId) {
        // The outlet wallet is msg.sender by design; deployer should enforce outlet ownership.
        // Cooldown (24h)
        uint64 nowTs = uint64(block.timestamp);
        if (nowTs - lastRequestAt[msg.sender] < 86400) revert PressErrors.Cooldown();
        lastRequestAt[msg.sender] = nowTs;

        (uint256 amt, uint256 fee, uint256 bond) = _preset(preset);
        requestId = ++requestCount;

        // Collect fee: 50% to treasury, 50% stays escrowed as bond in this contract.
        // Bond is additional and also escrowed. Governance may later slash bonds.
        require(pressToken.transferFrom(msg.sender, treasury, fee/2), "fee_treasury");
        require(pressToken.transferFrom(msg.sender, address(this), fee - (fee/2)), "fee_escrow");
        require(pressToken.transferFrom(msg.sender, address(this), bond), "bond_escrow");

        requests[requestId] = Request({
            outlet: msg.sender,
            token: outletToken,
            preset: preset,
            amount: amt,
            feePress: fee,
            bondPress: bond,
            createdAt: nowTs,
            finalized: false,
            approved: false
        });

        emit OutletMintRequested(requestId, msg.sender, outletToken, preset, amt, fee, bond);

        if (preset == Preset.LARGE) {
            // proposalHash is deterministic so off-chain governance can bind to it.
            bytes32 ph = keccak256(abi.encodePacked("PRESS_OUTLET_MINT", requestId, msg.sender, outletToken, amt));
            emit OutletMintProposalIntent(requestId, msg.sender, outletToken, amt, ph);
        }
    }

    // Finalize large request (called by governance executor module later).
    function finalize(uint256 requestId, bool approved) external {
        Request storage r = requests[requestId];
        if (r.outlet == address(0)) revert PressErrors.RequestNotFound();
        if (r.finalized) revert PressErrors.AlreadyFinalized();
        r.finalized = true;
        r.approved = approved;
        emit OutletMintFinalized(requestId, approved);
    }

    function _preset(Preset p) internal view returns (uint256 amt, uint256 fee, uint256 bond) {
        if (p == Preset.SMALL) return (mintSmall, feeSmall, bondSmall);
        if (p == Preset.MEDIUM) return (mintMedium, feeMedium, bondMedium);
        if (p == Preset.LARGE) return (mintLarge, feeLarge, bondLarge);
        revert PressErrors.InvalidPreset();
    }
}
