// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./PressParameters.sol";
import "./PressToken.sol";
import "./TruthEscrow.sol";
import "./ArticleApproval.sol";

contract ArticleRegistry {
    PressParameters public params;
    PressToken public press;
    TruthEscrow public escrow;
    ArticleApproval public approvals;

    struct Article {
        bytes32 articleId;
        uint64 createdTs;
        address outletOwner;
        address primaryAuthor;
        address coAuthor;
        uint256 escrowId;
        bool minted;
        string canonicalUrl;
    }

    mapping(bytes32 => Article) public articles;

    event ArticleRegistered(bytes32 indexed articleId, address indexed primaryAuthor, address outletOwner, string canonicalUrl);
    event CoAuthorAdded(bytes32 indexed articleId, address indexed coAuthor);
    event ArticleMinted(bytes32 indexed articleId, uint256 escrowId);

    constructor(address _params, address _press, address _escrow, address _approval){
        params = PressParameters(_params);
        press = PressToken(_press);
        escrow = TruthEscrow(_escrow);
        approvals = ArticleApproval(_approval);
    }

    function register(bytes32 articleId, address outletOwner, address primaryAuthor, string calldata canonicalUrl) external {
        Article storage a = articles[articleId];
        require(!a.minted && a.createdTs == 0, "exists");
        a.articleId = articleId;
        a.createdTs = uint64(block.timestamp);
        a.outletOwner = outletOwner;
        a.primaryAuthor = primaryAuthor;
        a.canonicalUrl = canonicalUrl;
        emit ArticleRegistered(articleId, primaryAuthor, outletOwner, canonicalUrl);
    }

    function addCoAuthor(bytes32 articleId, address coAuthor) external {
        Article storage a = articles[articleId];
        require(a.createdTs != 0, "missing");
        require(a.coAuthor == address(0), "already has");
        require(msg.sender == a.primaryAuthor, "only primary");
        uint256 fee = params.getUint(keccak256("coauthor_fee_wei"));
        if (fee == 0) fee = 100000000000000000; // 0.1 PRESS
        address treasury = params.getAddress(keccak256("treasury_wallet"));
        require(treasury != address(0), "treasury unset");
        press.transferFrom(msg.sender, treasury, fee);
        a.coAuthor = coAuthor;
        emit CoAuthorAdded(articleId, coAuthor);
    }

    function mintIfApproved(bytes32 articleId, bool openEscrow, uint256 escrowAmount) external {
        Article storage a = articles[articleId];
        require(a.createdTs != 0, "missing");
        require(!a.minted, "minted");

        (,,,bool finalized,bool approved) = approvals.getArticle(articleId);
        require(finalized && approved, "not approved");

        uint256 eid = 0;
        if (openEscrow) {
            uint256 min = params.getUint(keccak256("truth_escrow_min_wei"));
            if (min == 0) min = 1000000000000000000; // 1 PRESS
            require(escrowAmount >= min, "escrow too low");
            eid = escrow.open(articleId, msg.sender, escrowAmount);
        }
        a.escrowId = eid;
        a.minted = true;
        emit ArticleMinted(articleId, eid);
    }

    function revenueSplit(bytes32 articleId) external view returns (address primary, address coAuthor) {
        Article storage a = articles[articleId];
        return (a.primaryAuthor, a.coAuthor);
    }
}
