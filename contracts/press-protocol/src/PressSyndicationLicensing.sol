// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * PressSyndicationLicensing (RR182 rebuild)
 * PRESS-priced, parameterized licenses for Press articles.
 * Canonical events for Blockscout/custom explorers and indexers.
 */
contract PressSyndicationLicensing {
    enum UsageClass { WEB, PRINT, AI_TRAINING, RESEARCH, ARCHIVE }
    enum Scope { SINGLE_USE, LIMITED, UNLIMITED }
    enum Geo { GLOBAL, US, EU, APAC, CUSTOM }

    struct LicenseTerms {
        UsageClass usage;
        Scope scope;
        Geo geo;
        uint64 startsAt;
        uint64 endsAt;
        bool transferable;
        bytes32 customGeoHash;
        bytes32 customScopeHash;
    }

    struct License {
        bytes32 id;
        bytes32 articleId;
        address licensor;
        address licensee;
        uint256 pricePress;
        uint256 protocolCutBps;
        LicenseTerms terms;
        bool active;
    }

    mapping(bytes32 => License) public licenses;

    event LicenseIssued(
        bytes32 indexed licenseId,
        bytes32 indexed articleId,
        address indexed licensee,
        address licensor,
        uint256 pricePress,
        uint256 protocolCutBps,
        uint64 startsAt,
        uint64 endsAt,
        uint8 usage,
        uint8 scope,
        uint8 geo
    );

    event LicenseRenewed(bytes32 indexed licenseId, uint64 newEndsAt, uint256 renewalPricePress);
    event LicenseRevoked(bytes32 indexed licenseId, bytes32 reasonHash);
    event RoyaltyPaid(bytes32 indexed licenseId, address indexed to, uint256 amountPress, bytes32 routeRef);

    function issueLicense(
        bytes32 articleId,
        address licensor,
        address licensee,
        uint256 pricePress,
        uint256 protocolCutBps,
        LicenseTerms calldata terms
    ) external returns (bytes32 licenseId) {
        require(articleId != bytes32(0), "article");
        require(licensor != address(0) && licensee != address(0), "addr");
        require(pricePress > 0, "price");
        require(terms.endsAt > terms.startsAt, "time");
        require(protocolCutBps <= 2000, "cut");

        licenseId = keccak256(
            abi.encode(articleId, licensor, licensee, pricePress, protocolCutBps, terms.startsAt, terms.endsAt, terms.usage, terms.scope, terms.geo, block.chainid)
        );
        require(!licenses[licenseId].active, "exists");

        licenses[licenseId] = License({
            id: licenseId,
            articleId: articleId,
            licensor: licensor,
            licensee: licensee,
            pricePress: pricePress,
            protocolCutBps: protocolCutBps,
            terms: terms,
            active: true
        });

        emit LicenseIssued(
            licenseId, articleId, licensee, licensor, pricePress, protocolCutBps,
            terms.startsAt, terms.endsAt, uint8(terms.usage), uint8(terms.scope), uint8(terms.geo)
        );
    }

    function renewLicense(bytes32 licenseId, uint64 newEndsAt, uint256 renewalPricePress) external {
        License storage L = licenses[licenseId];
        require(L.active, "inactive");
        require(msg.sender == L.licensee, "licensee");
        require(newEndsAt > L.terms.endsAt, "ends");
        require(renewalPricePress > 0, "price");
        L.terms.endsAt = newEndsAt;
        emit LicenseRenewed(licenseId, newEndsAt, renewalPricePress);
    }

    function revokeLicense(bytes32 licenseId, bytes32 reasonHash) external {
        License storage L = licenses[licenseId];
        require(L.active, "inactive");
        require(msg.sender == L.licensor, "auth");
        L.active = false;
        emit LicenseRevoked(licenseId, reasonHash);
    }

    function emitRoyalty(bytes32 licenseId, address to, uint256 amountPress, bytes32 routeRef) external {
        require(licenses[licenseId].id != bytes32(0), "unknown");
        emit RoyaltyPaid(licenseId, to, amountPress, routeRef);
    }
}
