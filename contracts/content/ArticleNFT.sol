// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import "@openzeppelin/contracts/utils/Counters.sol";
import "../utils/AccessRoles.sol";
import "./OutletRegistry.sol";

contract ArticleNFT is ERC721URIStorage, AccessRoles {
    using Counters for Counters.Counter;
    Counters.Counter private _tokenIdCounter;

    struct Article {
        uint256 id;
        uint256 outletId;
        address author;
        string primaryURI;
        string mirrorURI;
        string coldStorageURI;
        int256 score;
        uint256 upvotes;
        uint256 downvotes;
        uint256 publishedAt;
    }

    OutletRegistry public outletRegistry;
    mapping(uint256 => Article) private _articles;
    mapping(uint256 => mapping(address => bool)) public hasVoted;

    event ArticleMinted(
        uint256 indexed id,
        uint256 indexed outletId,
        address indexed author,
        string primaryURI,
        string mirrorURI,
        string coldStorageURI,
        uint256 publishedAt
    );

    event ArticleStorageUpdated(
        uint256 indexed id,
        string primaryURI,
        string mirrorURI,
        string coldStorageURI
    );

    event ArticleVoted(
        uint256 indexed id,
        address indexed voter,
        bool upvote,
        int256 newScore,
        uint256 upvotes,
        uint256 downvotes,
        uint256 timestamp
    );

    constructor(address superAdmin, address outletRegistry_)
        ERC721("PressChain Article", "PRESSART")
        AccessRoles(superAdmin)
    {
        outletRegistry = OutletRegistry(outletRegistry_);
    }

    function mintArticle(
        uint256 outletId,
        string calldata primaryURI,
        string calldata mirrorURI,
        string calldata coldStorageURI,
        address author
    ) external onlyRole(JOURNALIST_ROLE) returns (uint256) {
        OutletRegistry.Outlet memory o = outletRegistry.getOutlet(outletId);
        require(o.active, "Outlet inactive");

        _tokenIdCounter.increment();
        uint256 tokenId = _tokenIdCounter.current();

        _safeMint(author, tokenId);
        _setTokenURI(tokenId, primaryURI);

        _articles[tokenId] = Article({
            id: tokenId,
            outletId: outletId,
            author: author,
            primaryURI: primaryURI,
            mirrorURI: mirrorURI,
            coldStorageURI: coldStorageURI,
            score: 0,
            upvotes: 0,
            downvotes: 0,
            publishedAt: block.timestamp
        });

        emit ArticleMinted(
            tokenId,
            outletId,
            author,
            primaryURI,
            mirrorURI,
            coldStorageURI,
            block.timestamp
        );
        return tokenId;
    }

    function updateStorageURIs(
        uint256 articleId,
        string calldata primaryURI,
        string calldata mirrorURI,
        string calldata coldStorageURI
    ) external {
        require(_exists(articleId), "No article");
        Article storage a = _articles[articleId];
        require(a.author == msg.sender || hasRole(SUPER_ADMIN_ROLE, msg.sender), "Not authorized");
        a.primaryURI = primaryURI;
        a.mirrorURI = mirrorURI;
        a.coldStorageURI = coldStorageURI;
        _setTokenURI(articleId, primaryURI);
        emit ArticleStorageUpdated(articleId, primaryURI, mirrorURI, coldStorageURI);
    }

    function voteUp(uint256 articleId) external {
        _vote(articleId, true);
    }

    function voteDown(uint256 articleId) external {
        _vote(articleId, false);
    }

    function _vote(uint256 articleId, bool up) internal {
        require(_exists(articleId), "No article");
        require(!hasVoted[articleId][msg.sender], "Already voted");
        Article storage a = _articles[articleId];
        hasVoted[articleId][msg.sender] = true;
        if (up) {
            a.upvotes += 1;
            a.score += 1;
        } else {
            a.downvotes += 1;
            a.score -= 1;
        }
        emit ArticleVoted(
            articleId,
            msg.sender,
            up,
            a.score,
            a.upvotes,
            a.downvotes,
            block.timestamp
        );
    }
}
