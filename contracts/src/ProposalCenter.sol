// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
}

interface IBondVault {
    function bonded(address who, bytes32 roleKey) external view returns (uint256);
}

interface IPressParameters {
    function params(bytes32 key) external view returns (uint256);
}

interface IOutletTokenFactoryExec {
    function executeMintFromProposal(bytes32 outletId, uint256 amount, address to) external;
}

interface IPressParametersMutable {
    function set(bytes32 key, uint256 value) external;
}

interface IRefundVault {
    function params(bytes32 key) external view returns (uint256);
}

interface IRefundVault {
    function pay(address to, uint256 amount) external;
}

/// @notice Proposal center with PRESS-token fees (ERC20), refund vault hardening, outlet discount,
/// and parameter-driven fee/duration values (changeable via PARAM_CHANGE governance).
contract ProposalCenter {
    enum ProposalType { VARIABLE_CHANGE, MAJOR_UPGRADE, GRANT, COURT_POLICY }

    struct Proposal {
        bytes32 kind;
        address proposer;
        ProposalType pType;
        string title;
        string description;
        bytes payload;
        uint64 start;
        uint64 end;
        uint256 forVotes;
        uint256 againstVotes;
        bool passed;
        bool executed;
        uint256 createFeePaid;
        string closeReason;
    }

    IERC20 public immutable press;
    address public treasury;

    IBondVault public bondVault;
    bytes32 public immutable ROLE_OUTLET_BOND = keccak256("OUTLET_BOND");

    IPressParameters public pressParams;
    IRefundVault public refundVault;

    address public councilExecutor;

    // Parameter keys (bytes32)
    bytes32 public constant P_VOTE_FEE = keccak256("proposal_vote_fee_press");
    bytes32 public constant P_CREATE_FEE = keccak256("proposal_create_fee_press");
    bytes32 public constant P_DURATION = keccak256("proposal_duration_seconds");
    bytes32 public constant P_MAX_DURATION = keccak256("proposal_max_duration_seconds");

    bytes32 public constant P_MIN_VOTES = keccak256("proposal_min_total_votes");
    bytes32 public constant P_YES_BPS = keccak256("proposal_yes_bps"); // pass threshold in basis points
    bytes32 public constant P_OUTLET_DISCOUNT_BPS = keccak256("proposal_outlet_create_discount_bps");

    bytes32 public constant P_REFUND_BPS = keccak256("proposal_refund_bps"); // 2500 = 25%
    bytes32 public constant P_REFUND_RESERVE_BPS = keccak256("proposal_refund_reserve_bps");

    // Proposal kinds
    bytes32 public constant K_PARAM_CHANGE = keccak256("PARAM_CHANGE");

    // Immediate execution thresholds for safe VARIABLE_CHANGE proposals
    bytes32 public constant P_EXEC_MIN_VOTES = keccak256("proposal_execute_min_total_votes");
    bytes32 public constant P_EXEC_YES_BPS = keccak256("proposal_execute_yes_bps");
 // portion escrowed to refund vault on create

    uint256 public proposalCount;
    mapping(uint256 => Proposal) public proposals;
    mapping(uint256 => mapping(address => bool)) public voted;

    event ProposalCreated(uint256 indexed id, bytes32 indexed kind, address indexed proposer, uint256 createFeePaid);
    event VoteCast(uint256 indexed id, address indexed voter, bool support, uint256 voteFeePaid);
    event ProposalFinalized(uint256 indexed id, bool passed, string closeReason);
    event ProposalExecuted(uint256 indexed id);
    event TreasurySet(address treasury);
    event BondVaultSet(address bondVault);
    event ParamsSet(address pressParams);
    event RefundVaultSet(address refundVault);
    event CouncilExecutorSet(address councilExecutor);

    constructor(address pressToken, address treasury_) {
        press = IERC20(pressToken);
        treasury = treasury_;
        emit TreasurySet(treasury_);
    }

    function setTreasury(address t) external {
        // In production: CouncilExecutor-only
        treasury = t;
        emit TreasurySet(t);
    }

    function setBondVault(address bv) external {
        bondVault = IBondVault(bv);
        emit BondVaultSet(bv);
    }

    function setPressParameters(address pp) external {
        pressParams = IPressParameters(pp);
        emit ParamsSet(pp);
    }

    function setRefundVault(address rv) external {
        refundVault = IRefundVault(rv);
        emit RefundVaultSet(rv);
    }

    function setCouncilExecutor(address ce) external {
        // In production: only Council / governance. Deployer wires at install.
        councilExecutor = ce;
        emit CouncilExecutorSet(ce);
    }

    function proposalPassed(uint256 id) external view returns (bool) { return proposals[id].passed; }
    function proposalExecuted(uint256 id) external view returns (bool) { return proposals[id].executed; }
    function getProposalPayload(uint256 id) external view returns (bytes memory) { return proposals[id].payload; }

    function _param(bytes32 k, uint256 fallback_) internal view returns (uint256) {
        address pp = address(pressParams);
        if(pp == address(0)) return fallback_;
        return pressParams.params(k);
    }

    function _isOutlet(address who) internal view returns (bool) {
        if(address(bondVault) == address(0)) return false;
        return bondVault.bonded(who, ROLE_OUTLET_BOND) > 0;
    }

    function _take(address from, address to, uint256 amount) internal {
        if(amount == 0) return;
        bool ok = press.transferFrom(from, to, amount);
        require(ok, "TRANSFER_FROM_FAIL");
    }

    function createProposal(bytes32 kind, string calldata title, string calldata description, bytes calldata payload, uint8 pType) external returns (uint256) {
        uint256 createFee = _param(P_CREATE_FEE, 250e18);
        if(_isOutlet(msg.sender)) {
            // outlet discount (bps)
            uint256 discBps = _param(P_OUTLET_DISCOUNT_BPS, 1000); // 10%
            if(discBps > 0 && discBps < 10000) {
                createFee = (createFee * (10000 - discBps)) / 10000;
            }
        }

        uint256 refundReserveBps = _param(P_REFUND_RESERVE_BPS, 2500); // 25% escrow by default
        uint256 reserve = (createFee * refundReserveBps) / 10000;
        uint256 toTreasury = createFee - reserve;

        _take(msg.sender, treasury, toTreasury);
        if(reserve > 0) {
            require(address(refundVault) != address(0), "REFUND_VAULT_REQUIRED");
            _take(msg.sender, address(refundVault), reserve);
        }

        uint256 duration = _param(P_DURATION, 7 days);
        uint256 maxDur = _param(P_MAX_DURATION, 21 days);
        if(duration > maxDur) duration = maxDur;

        proposalCount += 1;
        proposals[proposalCount] = Proposal({
            kind: kind,
            proposer: msg.sender,
            title: title,
            description: description,
            payload: payload,
            start: uint64(block.timestamp),
            end: uint64(block.timestamp + duration),
            forVotes: 0,
            againstVotes: 0,
            passed: false,
            executed: false,
            createFeePaid: createFee,
            closeReason: ""
        });

        emit ProposalCreated(proposalCount, kind, msg.sender, createFee);
        return proposalCount;
    }

    function vote(uint256 id, bool support) external {
        Proposal storage p = proposals[id];
        require(id > 0 && id <= proposalCount, "BAD_ID");
        require(block.timestamp >= p.start && block.timestamp < p.end, "VOTING_CLOSED");
        require(!voted[id][msg.sender], "ALREADY_VOTED");

        uint256 voteFee = _param(P_VOTE_FEE, 5e18);
// fee bands by proposal type (major upgrades cost more to vote on)
if(p.pType == ProposalType.MAJOR_UPGRADE) {
    voteFee = _param(P_VOTE_FEE_MAJOR, voteFee * 2);
} else if(p.pType == ProposalType.GRANT) {
    voteFee = _param(P_VOTE_FEE_GRANT, voteFee);
} else if(p.pType == ProposalType.COURT_POLICY) {
    voteFee = _param(P_VOTE_FEE_COURT, voteFee);
}
require(voteFee > 0, "VOTE_FEE_ZERO");
        _take(msg.sender, treasury, voteFee);

        voted[id][msg.sender] = true;
        if(support) p.forVotes += 1;
        else p.againstVotes += 1;

        emit VoteCast(id, msg.sender, support, voteFee);
    }

    function finalize(uint256 id, string calldata reason) external {
    Proposal storage p = proposals[id];
    require(id > 0 && id <= proposalCount, "BAD_ID");
    require(block.timestamp >= p.end, "NOT_ENDED");
    require(bytes(p.closeReason).length == 0, "ALREADY_FINAL");

    uint256 total = p.forVotes + p.againstVotes;
    uint256 minVotes = _param(P_MIN_VOTES, 250); // default quorum
    uint256 yesBps = _param(P_YES_BPS, 6000); // default: 60% yes

    // Higher thresholds for MAJOR_UPGRADE (e.g., outlet token mint proposals)
    if (p.pType == ProposalType.MAJOR_UPGRADE) {
        minVotes = _param(P_MIN_VOTES_MAJOR, 1500);
        yesBps = _param(P_YES_BPS_MAJOR, 7000);
    }

    bool passed = false;
    if(total >= minVotes && total > 0) {
        // yes share in bps
        uint256 yesShare = (p.forVotes * 10000) / total;
        if(yesShare >= yesBps) passed = true;
    }

    p.passed = passed;
    p.closeReason = reason;

    emit ProposalFinalizedDetailed(id, passed, reason, p.forVotes, p.againstVotes, total, minVotes, yesBps);

        if(p.passed) {
            uint256 refundBps = _param(P_REFUND_BPS, 2500); // 25%
            uint256 refund = (p.createFeePaid * refundBps) / 10000;

            if(refund > 0) {
                require(address(refundVault) != address(0), "REFUND_VAULT_REQUIRED");
                refundVault.pay(p.proposer, refund);
            }
        }
    }

    function finalizeIfExpired(uint256 id) external {
        Proposal storage p = proposals[id];
        require(block.timestamp >= p.end, "NOT_ENDED");
        if(bytes(p.closeReason).length == 0) {
            finalize(id, "AUTO_EXPIRED");
        }
    }

    function markExecuted(uint256 id) external {
        require(msg.sender == councilExecutor, "EXECUTOR_ONLY");
        proposals[id].executed = true;
        emit ProposalExecuted(id);
    }

/// @notice Execute a safe VARIABLE_CHANGE (PARAM_CHANGE) proposal immediately once passed,
/// provided it meets the stricter "execute now" thresholds.
/// NOTE: PressParameters executor should be set to this ProposalCenter contract at install time,
/// while ProposalCenter.councilExecutor should be the council multisig.
function executeParamChange(uint256 id) external {
    Proposal storage p = proposals[id];
    require(id > 0 && id <= proposalCount, "BAD_ID");
    require(p.passed, "NOT_PASSED");
    require(!p.executed, "ALREADY_EXEC");
    require(p.pType == ProposalType.VARIABLE_CHANGE, "NOT_VAR");
    require(p.kind == K_PARAM_CHANGE, "NOT_PARAM_KIND");
    require(msg.sender == councilExecutor, "EXECUTOR_ONLY");

    uint256 total = p.forVotes + p.againstVotes;
    uint256 minVotes = _param(P_EXEC_MIN_VOTES, 500);
    uint256 yesBps = _param(P_EXEC_YES_BPS, 7500);

    require(total >= minVotes && total > 0, "EXEC_QUORUM");
    uint256 yesShare = (p.forVotes * 10000) / total;
    require(yesShare >= yesBps, "EXEC_SUPPORT");

    (bytes32 key, uint256 value) = abi.decode(p.payload, (bytes32, uint256));
    require(address(pressParams) != address(0), "NO_PARAMS");
    IPressParametersMutable(address(pressParams)).set(key, value);

    p.executed = true;
    emit ParamChangeExecuted(id, key, value, msg.sender);
}


function executeOutletTokenMint(uint256 id, address outletTokenFactory, address to) external {
    Proposal storage p = proposals[id];
    require(id > 0 && id <= proposalCount, "BAD_ID");
    require(p.passed, "NOT_PASSED");
    require(!p.executed, "ALREADY_EXEC");
    require(p.kind == bytes32("OUTLET_TOKEN_MINT"), "NOT_MINT_KIND");
    require(msg.sender == councilExecutor, "EXECUTOR_ONLY");

    // Very high support requirements for mint execution
    uint256 total = p.forVotes + p.againstVotes;
    uint256 minVotes = _param(P_EXEC_MIN_VOTES, 500);
    uint256 yesBps = _param(P_EXEC_YES_BPS, 7500);

    // For mint proposals (pType MAJOR_UPGRADE), require higher execution thresholds too:
    if (p.pType == ProposalType.MAJOR_UPGRADE) {
        minVotes = _param(P_MIN_VOTES_MAJOR, 1500);
        yesBps = _param(P_YES_BPS_MAJOR, 7000);
    }

    require(total >= minVotes && total > 0, "EXEC_QUORUM");
    uint256 yesShare = (p.forVotes * 10000) / total;
    require(yesShare >= yesBps, "EXEC_SUPPORT");

    (bytes32 outletId, uint256 amount) = abi.decode(p.payload, (bytes32, uint256));
    require(outletTokenFactory != address(0), "NO_FACTORY");
    require(to != address(0), "NO_TO");
    IOutletTokenFactoryExec(outletTokenFactory).executeMintFromProposal(outletId, amount, to);

    p.executed = true;
    emit ParamChangeExecuted(id, bytes32("OUTLET_TOKEN_MINT"), amount, msg.sender);
}

}
