// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title PressGovernance
/// @notice Fee-based proposals + fee-based voting (never free) with explorer-friendly events.
///         Adds: (1) 25% proposal-fee refund on approval, (2) on-chain close reason,
///         (3) whitelisted "easy variable" auto-apply gated by council multisig executor.
interface IERC20 {
    function transferFrom(address from, address to, uint256 value) external returns (bool);
    function allowance(address owner, address spender) external view returns (uint256);
}
interface IPressUpgradeQueue {
    function queue(bytes32 batchId, uint256 proposalId, bytes32 configKey, int256 configValue) external;
}

contract PressGovernance {
    event ProposalCreated(
        uint256 indexed id,
        address indexed proposer,
        string title,
        bytes32 configKey,
        int256 configValue,
        uint256 feePaid,
        uint256 createdAt,
        uint256 endsAt
    );
    event VoteCast(uint256 indexed id, address indexed voter, bool support, uint256 weight, uint256 feePaid);
    event VoteFeeCharged(address indexed voter, uint256 amount);
    event GrantExecuted(uint256 indexed id, address indexed recipient, uint256 amountWei);
    event ProposalFinalized(uint256 indexed id, bool passed, uint256 yesVotes, uint256 noVotes, string reason, uint256 finalizedAt, bool autoApplied, uint256 refundPaid);

    enum ActionType {
    CONFIG_CHANGE,      // easy variable
    UPGRADE_BATCH,      // queued upgrade item
    GRANT,              // treasury grant payout
    COUNCIL_ELECTION,   // council role changes
    COURT_POLICY        // court parameter changes
}

    struct Threshold {
    uint32 minVoters;      // minimum distinct voters to finalize
    uint32 quorumVotes;    // minimum total votes (yes+no) to finalize
    uint16 yesBps;         // minimum yes percentage in basis points (e.g. 6000 = 60%)
    uint32 minDurationSec; // minimum voting duration before finalize unless supermajority
    uint16 superYesBps;    // early finalize if yes >= superYesBps and minVoters met
}

    struct Proposal {
        address proposer;
        string title;
        bytes32 configKey;
        ActionType actionType;
        int256 configValue;
        uint256 feePaid;
        uint256 createdAt;
        uint256 endsAt;
        uint256 yesVotes;
        uint256 noVotes;
        uint32 voterCount;
        uint64 createdAt;
        bool finalized;
        bool passed;
        string reason;
        bool autoApplied;
        uint256 refundPaid;
    }

    uint256 public proposalFeeWei;
    uint256 public voteFeeWei;
    uint256 public voteWeight;

    // Council executor (multisig) required to finalize proposals and apply whitelisted variable changes.
    address public councilExecutor;
    address public upgradeQueue;
    address public pressToken;
    uint256 public voteFeePress; // flat fee per vote in PRESS token smallest units
    address public treasury;

    mapping(bytes32 => bool) public configKeyBatch;

    Threshold public defaultThreshold;
    mapping(bytes32 => Threshold) public thresholdByKey;
    mapping(ActionType => Threshold) public thresholdByAction;

    mapping(uint256 => Proposal) public proposals;
    mapping(uint256 => mapping(address => bool)) public voted;
    uint256 public nextId = 1;

    // Governance variables store (easy-to-change variables only).
    mapping(bytes32 => int256) public configVars;
    mapping(bytes32 => bool) public configKeyWhitelisted;

    constructor(uint256 _proposalFeeWei, uint256 _voteFeeWei, address _councilExecutor) {
        proposalFeeWei = _proposalFeeWei;
        voteFeeWei = _voteFeeWei;
        voteWeight = 1;
        councilExecutor = _councilExecutor;
    }

    modifier onlyCouncilExecutor() {
        require(msg.sender == councilExecutor, "ONLY_COUNCIL_EXECUTOR");
        _;
    }

    function setFees(uint256 _proposalFeeWei, uint256 _voteFeeWei) external onlyCouncilExecutor {
        proposalFeeWei = _proposalFeeWei;
        voteFeeWei = _voteFeeWei;
    }

    function setCouncilExecutor(address _councilExecutor) external onlyCouncilExecutor {
        require(_councilExecutor != address(0), "ZERO_ADDRESS");
        councilExecutor = _councilExecutor;
    }

    /// @notice whitelist keys that are safe for auto-apply (e.g., role costs, vote thresholds, fees).
    function setTreasury(address t) external onlyCouncilExecutor {
        require(t != address(0), "ZERO_ADDRESS");
        treasury = t;
    }

    function setPressToken(address t) external onlyCouncilExecutor {
        require(t != address(0), "ZERO_ADDRESS");
        pressToken = t;
    }

    function setDefaultThreshold(uint32 minVoters, uint32 quorumVotes, uint16 yesBps, uint32 minDurationSec, uint16 superYesBps) external onlyCouncilExecutor {
    require(yesBps <= 10000 && superYesBps <= 10000, "BPS");
    defaultThreshold = Threshold(minVoters, quorumVotes, yesBps, minDurationSec, superYesBps);
}

function setThresholdForAction(uint8 actionType, uint32 minVoters, uint32 quorumVotes, uint16 yesBps, uint32 minDurationSec, uint16 superYesBps) external onlyCouncilExecutor {
    require(actionType <= uint8(ActionType.COURT_POLICY), "BAD_ACTION");
    require(yesBps <= 10000 && superYesBps <= 10000, "BPS");
    thresholdByAction[ActionType(actionType)] = Threshold(minVoters, quorumVotes, yesBps, minDurationSec, superYesBps);
}

function setThresholdForKey(bytes32 key, uint32 minVoters, uint32 quorumVotes, uint16 yesBps, uint32 minDurationSec, uint16 superYesBps) external onlyCouncilExecutor {
    require(yesBps <= 10000 && superYesBps <= 10000, "BPS");
    thresholdByKey[key] = Threshold(minVoters, quorumVotes, yesBps, minDurationSec, superYesBps);
}

function setVoteFeePress(uint256 fee) external onlyCouncilExecutor {
        voteFeePress = fee;
    }

    function setUpgradeQueue(address q) external onlyCouncilExecutor {
        upgradeQueue = q;
    }

    /// @notice mark keys as batch-upgrade keys (queued into monthly release batch when approved)
    function setBatchKey(bytes32 key, bool isBatch) external onlyCouncilExecutor {
        configKeyBatch[key] = isBatch;
    }

    function whitelistConfigKey(bytes32 key, bool allowed) external onlyCouncilExecutor {
        configKeyWhitelisted[key] = allowed;
    }

    function createGrantProposal(address recipient, uint256 amountWei) external returns (uint256) {
    require(recipient != address(0), "ZERO_RECIPIENT");
    uint256 id = createProposal(bytes32("GRANT"), amountWei);
    proposals[id].actionType = ActionType.GRANT;
    grants[id] = GrantRequest({recipient: recipient, amountWei: amountWei, executed: false});
    return id;
}

function createProposalWithAction(bytes32 key, uint256 newValue, uint8 actionType) external returns (uint256) {
    require(actionType <= uint8(ActionType.COURT_POLICY), "BAD_ACTION");
    uint256 id = createProposal(key, newValue);
    proposals[id].actionType = ActionType(actionType);
    return id;
}

function createProposal(
        string calldata title,
        bytes32 configKey,
        int256 configValue,
        uint256 durationSeconds
    ) external payable returns (uint256 id) {
        require(msg.value >= proposalFeeWei, "PROPOSAL_FEE_REQUIRED");
        require(durationSeconds >= 3600, "DURATION_TOO_SHORT");
        id = nextId++;
        proposals[id] = Proposal({
            proposer: msg.sender,
            title: title,
            configKey: configKey,
            configValue: configValue,
            feePaid: msg.value,
            createdAt: block.timestamp,
            endsAt: block.timestamp + durationSeconds,
            yesVotes: 0,
            noVotes: 0,
            finalized: false,
            passed: false,
            reason: "",
            autoApplied: false,
            refundPaid: 0
        });
        emit ProposalCreated(id, msg.sender, title, configKey, configValue, msg.value, block.timestamp, block.timestamp + durationSeconds);
    }

    function vote(uint256 id, bool support) external payable {
        Proposal storage p = proposals[id];
        require(p.proposer != address(0), "PROPOSAL_NOT_FOUND");
        require(block.timestamp < p.endsAt, "VOTING_CLOSED");
        require(!voted[id][msg.sender], "ALREADY_VOTED");
        require(msg.value >= voteFeeWei, "VOTE_FEE_REQUIRED");
        voted[id][msg.sender] = true;

        if (support) p.yesVotes += voteWeight;
        else p.noVotes += voteWeight;

        emit VoteCast(id, msg.sender, support, voteWeight, msg.value);
    }

    /// @notice finalize must be executed by council executor (multisig),
    ///         ensuring a public, accountable finalization action on-chain.
    ///         If passed and configKey is whitelisted, auto-applies configVars[configKey] = configValue.
    ///         Refunds 25% of proposal fee to proposer if passed.
    function finalize(uint256 id, string calldata reason) external onlyCouncilExecutor {
        Proposal storage p = proposals[id];
        require(p.proposer != address(0), "PROPOSAL_NOT_FOUND");
        require(block.timestamp >= p.endsAt, "VOTING_OPEN");
        require(!p.finalized, "FINALIZED");

        p.finalized = true;
        p.passed = p.yesVotes > p.noVotes;
        p.reason = reason;

        bool applied = false;
        if (p.passed && configKeyWhitelisted[p.configKey]) {
            configVars[p.configKey] = p.configValue;
            applied = true;
            p.autoApplied = true;
        }

        // If this is a batch key and not auto-applied, emit to upgrade queue (explorer friendly)
if (p.passed && !applied && configKeyBatch[p.configKey] && upgradeQueue != address(0)) {
    // monthly batchId (YYYY-MM) as bytes32
    // Example: "2025-12" => keccak256 to bytes32 (deterministic)
    bytes32 batchId = keccak256(abi.encodePacked(_monthKey()));
    IPressUpgradeQueue(upgradeQueue).queue(batchId, id, p.configKey, p.configValue);
}

uint256 refund = 0;

        if (p.passed) {
            refund = p.feePaid / 4; // 25%
            p.refundPaid = refund;
            if (refund > 0 && address(this).balance >= refund) {
                (bool ok,) = p.proposer.call{value: refund}("");
                if (!ok) {
                    // if refund fails, keep refundPaid recorded as 0
                    p.refundPaid = 0;
                    refund = 0;
                }
            } else {
                p.refundPaid = 0;
                refund = 0;
            }
        }

        emit ProposalFinalized(id, p.passed, p.yesVotes, p.noVotes, reason, block.timestamp, applied, refund);
    }

    /// @notice emergency close (e.g., proposal invalid) — also records reason on-chain.
    function closeWithoutPass(uint256 id, string calldata reason) external onlyCouncilExecutor {
        Proposal storage p = proposals[id];
        require(p.proposer != address(0), "PROPOSAL_NOT_FOUND");
        require(!p.finalized, "FINALIZED");
        p.finalized = true;
        p.passed = false;
        p.reason = reason;
        emit ProposalFinalized(id, false, p.yesVotes, p.noVotes, reason, block.timestamp, false, 0);
    }

    receive() external payable {}
}
