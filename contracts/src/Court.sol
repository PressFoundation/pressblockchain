// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

contract Court {
    enum CaseType { Dispute, Abuse, Fraud, IP, Other }
    enum CaseStatus { Open, InReview, Resolved }

    struct Case {
        address filedBy;
        uint256 outletId;
        CaseType caseType;
        string evidenceURI;
        uint64 createdAt;
        CaseStatus status;
    }

    uint256 public caseCount;
    mapping(uint256 => Case) public cases;

    event CaseFiled(uint256 indexed caseId, uint256 indexed outletId, address indexed filedBy, CaseType caseType, string evidenceURI);
    event CaseStatusChanged(uint256 indexed caseId, CaseStatus status);

    function fileCase(uint256 outletId, CaseType caseType, string calldata evidenceURI) external returns (uint256) {
        caseCount++;
        uint256 id = caseCount;
        cases[id] = Case({
            filedBy: msg.sender,
            outletId: outletId,
            caseType: caseType,
            evidenceURI: evidenceURI,
            createdAt: uint64(block.timestamp),
            status: CaseStatus::Open
        });
        emit CaseFiled(id, outletId, msg.sender, caseType, evidenceURI);
        return id;
    }

    function setStatus(uint256 caseId, CaseStatus status) external {
        // baseline: permissioning added in governance pass
        cases[caseId].status = status;
        emit CaseStatusChanged(caseId, status);
    }
}
