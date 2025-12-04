// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../utils/AccessRoles.sol";
import "../identity/PressID.sol";

contract CourtModule is AccessRoles {
    enum CaseState { Pending, JurySummoned, VotingOpen, VotingClosed, Resolved }

    struct Case {
        uint256 id;
        uint256 articleId;
        address filer;
        string reasonURI;
        uint256 createdAt;
        uint256 votingStartsAt;
        uint256 votingEndsAt;
        uint256 votesFor;
        uint256 votesAgainst;
        CaseState state;
        bool verdict;
    }

    PressID public pressId;
    uint256 public nextCaseId;
    mapping(uint256 => Case) public cases;
    mapping(uint256 => address[]) public caseJurors;
    mapping(uint256 => mapping(address => bool)) public juryVoted;

    uint256 public minJurors = 7;
    uint256 public maxJurors = 21;
    uint256 public votingWindow = 3 days;

    event CaseFiled(uint256 indexed id, uint256 indexed articleId, address indexed filer, string reasonURI);
    event JurorsSummoned(uint256 indexed caseId, address[] jurors);
    event VotingOpened(uint256 indexed caseId, uint256 startsAt, uint256 endsAt);
    event JuryVoteCast(uint256 indexed caseId, address indexed juror, bool support);
    event VotingClosed(uint256 indexed caseId);
    event CaseResolved(uint256 indexed caseId, bool verdict);
    event TrustRewarded(address indexed user, uint256 amount);

    constructor(address superAdmin, address pressId_)
        AccessRoles(superAdmin)
    {
        pressId = PressID(pressId_);
        _grantRole(TRUST_ENGINE_ROLE, address(this));
    }

    function fileDispute(uint256 articleId, string calldata reasonURI) external returns (uint256) {
        uint256 id = ++nextCaseId;
        cases[id] = Case({
            id: id,
            articleId: articleId,
            filer: msg.sender,
            reasonURI: reasonURI,
            createdAt: block.timestamp,
            votingStartsAt: 0,
            votingEndsAt: 0,
            votesFor: 0,
            votesAgainst: 0,
            state: CaseState.Pending,
            verdict: false
        });
        emit CaseFiled(id, articleId, msg.sender, reasonURI);
        return id;
    }

    function summonJurors(uint256 caseId, address[] calldata jurors)
        external
        onlyRole(COURT_ADMIN_ROLE)
    {
        Case storage c = cases[caseId];
        require(c.state == CaseState.Pending, "Not pending");
        require(jurors.length >= minJurors && jurors.length <= maxJurors, "invalid jury size");
        for (uint256 i = 0; i < jurors.length; i++) {
            require(hasRole(JURY_ROLE, jurors[i]), "not juror");
            caseJurors[caseId].push(jurors[i]);
        }
        c.state = CaseState.JurySummoned;
        emit JurorsSummoned(caseId, jurors);
    }

    function openVoting(uint256 caseId)
        external
        onlyRole(COURT_ADMIN_ROLE)
    {
        Case storage c = cases[caseId];
        require(c.state == CaseState.JurySummoned, "must be summoned");
        c.votingStartsAt = block.timestamp;
        c.votingEndsAt = block.timestamp + votingWindow;
        c.state = CaseState.VotingOpen;
        emit VotingOpened(caseId, c.votingStartsAt, c.votingEndsAt);
    }

    function juryVote(uint256 caseId, bool support) external {
        Case storage c = cases[caseId];
        require(c.state == CaseState.VotingOpen, "voting closed");
        require(block.timestamp <= c.votingEndsAt, "expired");
        require(_isCaseJuror(caseId, msg.sender), "not case juror");
        require(!juryVoted[caseId][msg.sender], "already voted");
        juryVoted[caseId][msg.sender] = true;
        if (support) c.votesFor += 1;
        else c.votesAgainst += 1;
        emit JuryVoteCast(caseId, msg.sender, support);
    }

    function closeVoting(uint256 caseId)
        external
        onlyRole(COURT_ADMIN_ROLE)
    {
        Case storage c = cases[caseId];
        require(c.state == CaseState.VotingOpen, "not open");
        require(block.timestamp >= c.votingEndsAt, "too early");
        c.state = CaseState.VotingClosed;
        emit VotingClosed(caseId);
    }

    function resolveCase(uint256 caseId)
        external
        onlyRole(COURT_ADMIN_ROLE)
    {
        Case storage c = cases[caseId];
        require(c.state == CaseState.VotingClosed, "must be closed");
        bool verdict = c.votesFor >= c.votesAgainst;
        c.verdict = verdict;
        c.state = CaseState.Resolved;
        address[] memory jurors = caseJurors[caseId];
        for (uint256 i = 0; i < jurors.length; i++) {
            address j = jurors[i];
            if (!juryVoted[caseId][j]) continue;
            pressId.adjustTrust(j, int256(2));
            emit TrustRewarded(j, 2);
        }
        emit CaseResolved(caseId, verdict);
    }

    function _isCaseJuror(uint256 caseId, address who) internal view returns (bool) {
        address[] memory jurors = caseJurors[caseId];
        for (uint256 i = 0; i < jurors.length; i++) {
            if (jurors[i] == who) return true;
        }
        return false;
    }
}
