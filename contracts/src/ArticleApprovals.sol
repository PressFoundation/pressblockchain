// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

interface IPressParameters {
    function params(bytes32 key) external view returns (uint256);
}

interface IOutletRegistry {
    function hasRole(bytes32 outletId, address account, bytes32 role) external view returns (bool);
}

interface ICouncilRegistry {
    // public getter for members mapping is expected to exist; we only require `active` + termEnd.
    function members(address account) external view returns (bool active, uint64 termStart, uint64 termEnd);
}

/// @notice Role-aware article approval voting with a hard 72h voting window.
/// Articles are immutable; approval status becomes an additional on-chain signal.
/// No vote changes; no double voting.
/// Anti-gatekeeping: approval requires minimum approvals from multiple role buckets.
contract ArticleApprovals {
    // Parameter keys (PressParameters)
    bytes32 public constant P_VOTE_WINDOW_SECS = keccak256("article_vote_window_seconds"); // default 72h
    bytes32 public constant P_COMMUNITY_APPROVALS = keccak256("article_community_approvals_min");
    bytes32 public constant P_OUTLET_APPROVALS = keccak256("article_outlet_approvals_min");
    bytes32 public constant P_COUNCIL_APPROVALS = keccak256("article_council_approvals_min");
    bytes32 public constant P_FLAG_MAX = keccak256("article_flags_max");

    // vote cost (in PRESS wei) by action
    bytes32 public constant P_VOTE_FEE_COMMUNITY = keccak256("article_vote_fee_community_press_wei");
    bytes32 public constant P_VOTE_FEE_OUTLET = keccak256("article_vote_fee_outlet_press_wei");
    bytes32 public constant P_VOTE_FEE_COUNCIL = keccak256("article_vote_fee_council_press_wei");

    // outlet roles that count as "outlet approvals"
    bytes32 public constant ROLE_MANAGER = keccak256("OUTLET_MANAGER");
    bytes32 public constant ROLE_EDITOR = keccak256("OUTLET_EDITOR");
    bytes32 public constant ROLE_WRITER = keccak256("OUTLET_WRITER");

    address public immutable pressToken;
    IPressParameters public immutable pressParams;
    IOutletRegistry public immutable outletRegistry;
    ICouncilRegistry public immutable councilRegistry;
    address public immutable treasury;

    struct VoteState {
        uint64 startAt;
        uint64 endAt;
        uint32 communityApprovals;
        uint32 outletApprovals;
        uint32 councilApprovals;
        uint32 flags;
        bool finalized;
        bool approved;
    }

    // articleId => state
    mapping(uint256 => VoteState) public voteStates;
    // articleId => voter => voted?
    mapping(uint256 => mapping(address => bool)) public voted;

    event ArticleVoteOpened(uint256 indexed articleId, uint64 startAt, uint64 endAt);
    event ArticleVoted(uint256 indexed articleId, address indexed voter, bool approve, uint8 bucket, uint256 feePaid);
    event ArticleVoteFinalized(uint256 indexed articleId, bool approved, uint32 community, uint32 outlet, uint32 council, uint32 flags);

    constructor(address _pressToken, address _pressParams, address _outletRegistry, address _councilRegistry, address _treasury) {
        pressToken = _pressToken;
        pressParams = IPressParameters(_pressParams);
        outletRegistry = IOutletRegistry(_outletRegistry);
        councilRegistry = ICouncilRegistry(_councilRegistry);
        treasury = _treasury;
    }

    function _param(bytes32 k, uint256 d) internal view returns (uint256) {
        uint256 v = pressParams.params(k);
        return v == 0 ? d : v;
    }

    function openIfNeeded(uint256 articleId) public {
        VoteState storage s = voteStates[articleId];
        if(s.startAt != 0) return;

        uint256 win = _param(P_VOTE_WINDOW_SECS, 72 hours);
        s.startAt = uint64(block.timestamp);
        s.endAt = uint64(block.timestamp + win);

        emit ArticleVoteOpened(articleId, s.startAt, s.endAt);
    }

    function _bucket(bytes32 outletId, address voter) internal view returns (uint8 bucket) {
        // 3=council, 2=outlet, 1=community
        (bool active,, uint64 termEnd) = councilRegistry.members(voter);
        if(active && termEnd >= block.timestamp) return 3;

        if(
            outletRegistry.hasRole(outletId, voter, ROLE_MANAGER) ||
            outletRegistry.hasRole(outletId, voter, ROLE_EDITOR) ||
            outletRegistry.hasRole(outletId, voter, ROLE_WRITER)
        ) return 2;

        return 1;
    }

    function _fee(uint8 bucket) internal view returns (uint256) {
        if(bucket == 3) return _param(P_VOTE_FEE_COUNCIL, 5e18);
        if(bucket == 2) return _param(P_VOTE_FEE_OUTLET, 2e18);
        return _param(P_VOTE_FEE_COMMUNITY, 1e18);
    }

    function vote(uint256 articleId, bytes32 outletId, bool approve) external {
        openIfNeeded(articleId);

        VoteState storage s = voteStates[articleId];
        require(block.timestamp <= s.endAt, "VOTE_ENDED");
        require(!voted[articleId][msg.sender], "ALREADY_VOTED");

        uint8 bucket = _bucket(outletId, msg.sender);
        uint256 fee = _fee(bucket);
        require(fee > 0, "FEE_ZERO");

        // collect fee to treasury (anti-spam)
        // minimal IERC20 transferFrom interface
        (bool ok, bytes memory data) = pressToken.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", msg.sender, treasury, fee));
        require(ok && (data.length == 0 || abi.decode(data, (bool))), "FEE_FAIL");

        voted[articleId][msg.sender] = true;

        if(approve) {
            if(bucket == 3) s.councilApprovals += 1;
            else if(bucket == 2) s.outletApprovals += 1;
            else s.communityApprovals += 1;
        } else {
            // flags are non-censoring "risk signals" only; they increase economic friction elsewhere
            s.flags += 1;
        }

        emit ArticleVoted(articleId, msg.sender, approve, bucket, fee);

        // optional auto-finalize if thresholds met early
        _tryFinalize(articleId);
    }

    function _tryFinalize(uint256 articleId) internal {
        VoteState storage s = voteStates[articleId];
        if(s.finalized) return;

        uint256 reqCommunity = _param(P_COMMUNITY_APPROVALS, 200);
        uint256 reqOutlet = _param(P_OUTLET_APPROVALS, 10);
        uint256 reqCouncil = _param(P_COUNCIL_APPROVALS, 3);
        uint256 maxFlags = _param(P_FLAG_MAX, 50);

        bool ok = (s.communityApprovals >= reqCommunity) && (s.outletApprovals >= reqOutlet) && (s.councilApprovals >= reqCouncil) && (s.flags <= maxFlags);
        if(ok) {
            s.finalized = true;
            s.approved = true;
            emit ArticleVoteFinalized(articleId, true, s.communityApprovals, s.outletApprovals, s.councilApprovals, s.flags);
        }
    }

    function finalize(uint256 articleId) external {
        VoteState storage s = voteStates[articleId];
        require(s.startAt != 0, "NOT_OPEN");
        require(!s.finalized, "FINAL");
        require(block.timestamp > s.endAt, "NOT_ENDED");

        uint256 reqCommunity = _param(P_COMMUNITY_APPROVALS, 200);
        uint256 reqOutlet = _param(P_OUTLET_APPROVALS, 10);
        uint256 reqCouncil = _param(P_COUNCIL_APPROVALS, 3);
        uint256 maxFlags = _param(P_FLAG_MAX, 50);

        bool ok = (s.communityApprovals >= reqCommunity) && (s.outletApprovals >= reqOutlet) && (s.councilApprovals >= reqCouncil) && (s.flags <= maxFlags);
        s.finalized = true;
        s.approved = ok;

        emit ArticleVoteFinalized(articleId, ok, s.communityApprovals, s.outletApprovals, s.councilApprovals, s.flags);
    }

    function getCounts(uint256 articleId) external view returns (uint64 startAt, uint64 endAt, uint32 community, uint32 outlet, uint32 council, uint32 flags, bool finalized, bool approved) {
        VoteState memory s = voteStates[articleId];
        return (s.startAt, s.endAt, s.communityApprovals, s.outletApprovals, s.councilApprovals, s.flags, s.finalized, s.approved);
    }
}
