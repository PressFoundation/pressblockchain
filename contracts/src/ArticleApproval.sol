// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./PressParameters.sol";
import "./PressToken.sol";

contract ArticleApproval {
    PressParameters public params;
    PressToken public press;

    enum Role { Reader, Journalist, Editor, Outlet, Council }

    struct VoteTally { uint64 yes; uint64 no; }

    struct ArticleVote {
        address author;
        uint64 startTs;
        uint64 endTs;
        bool finalized;
        bool approved;
        mapping(uint8 => VoteTally) tallies;
        mapping(address => bool) voted;
    }

    mapping(bytes32 => ArticleVote) private votes;

    event ArticleVoteOpened(bytes32 indexed articleId, address indexed author, uint64 startTs, uint64 endTs);
    event Voted(bytes32 indexed articleId, address indexed voter, uint8 role, bool support);
    event Finalized(bytes32 indexed articleId, bool approved);

    constructor(address _params, address _press) {
        params = PressParameters(_params);
        press = PressToken(_press);
    }

    function open(bytes32 articleId, address author) external {
        ArticleVote storage a = votes[articleId];
        require(a.startTs == 0, "already opened");
        a.author = author;
        a.startTs = uint64(block.timestamp);
        uint64 dur = uint64(params.getUint(keccak256("article_vote_duration_sec")));
        if (dur == 0) dur = 72 hours;
        a.endTs = uint64(block.timestamp) + dur;
        emit ArticleVoteOpened(articleId, author, a.startTs, a.endTs);
    }

    function vote(bytes32 articleId, uint8 role, bool support) external {
        ArticleVote storage a = votes[articleId];
        require(a.startTs != 0, "not opened");
        require(block.timestamp <= a.endTs, "voting ended");
        require(!a.voted[msg.sender], "already voted");
        a.voted[msg.sender] = true;

        uint256 fee = params.getUint(keccak256("article_vote_fee_wei"));
        if (fee > 0) {
            address treasury = params.getAddress(keccak256("treasury_wallet"));
            require(treasury != address(0), "treasury unset");
            press.transferFrom(msg.sender, treasury, fee);
        }

        VoteTally storage t = a.tallies[role];
        if (support) t.yes += 1;
        else t.no += 1;

        emit Voted(articleId, msg.sender, role, support);
    }

    function finalize(bytes32 articleId) external {
        ArticleVote storage a = votes[articleId];
        require(a.startTs != 0, "not opened");
        require(!a.finalized, "finalized");
        require(block.timestamp > a.endTs, "still active");
        bool ok = _meetsThresholds(a);
        a.finalized = true;
        a.approved = ok;
        emit Finalized(articleId, ok);
    }

    function getArticle(bytes32 articleId) external view returns (address author,uint64 startTs,uint64 endTs,bool finalized,bool approved) {
        ArticleVote storage a = votes[articleId];
        return (a.author, a.startTs, a.endTs, a.finalized, a.approved);
    }

    function getTally(bytes32 articleId, uint8 role) external view returns (uint64 yes, uint64 no) {
        ArticleVote storage a = votes[articleId];
        VoteTally storage t = a.tallies[role];
        return (t.yes, t.no);
    }

    function _meetsThresholds(ArticleVote storage a) internal view returns (bool) {
        uint256 r = params.getUint(keccak256("article_thr_reader_yes"));
        uint256 j = params.getUint(keccak256("article_thr_journalist_yes"));
        uint256 e = params.getUint(keccak256("article_thr_editor_yes"));
        uint256 o = params.getUint(keccak256("article_thr_outlet_yes"));
        uint256 c = params.getUint(keccak256("article_thr_council_yes"));

        if (r == 0) r = 25;
        if (j == 0) j = 5;
        if (e == 0) e = 2;
        if (o == 0) o = 1;
        if (c == 0) c = 0;

        if (a.tallies[uint8(Role.Reader)].yes < r) return false;
        if (a.tallies[uint8(Role.Journalist)].yes < j) return false;
        if (a.tallies[uint8(Role.Editor)].yes < e) return false;
        if (a.tallies[uint8(Role.Outlet)].yes < o) return false;
        if (c > 0 && a.tallies[uint8(Role.Council)].yes < c) return false;

        uint256 maxNo = params.getUint(keccak256("article_max_no_ratio_bps"));
        if (maxNo == 0) maxNo = 15000;
        for (uint8 role=0; role<5; role++) {
            VoteTally storage t = a.tallies[role];
            if (t.yes == 0 && t.no > 0) return false;
            if (t.yes > 0) {
                if (uint256(t.no) * 10000 > uint256(t.yes) * maxNo) return false;
            }
        }
        return true;
    }
}
