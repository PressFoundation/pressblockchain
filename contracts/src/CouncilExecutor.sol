// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IProposalCenter {
    function proposalPassed(uint256 proposalId) external view returns (bool);
    function proposalExecuted(uint256 proposalId) external view returns (bool);
    function markExecuted(uint256 proposalId) external;
    function getProposalPayload(uint256 proposalId) external view returns (bytes memory);
}

interface IPressParameters {
    function set(bytes32 key, uint256 value) external;
    function params(bytes32 key) external view returns (uint256);
}

interface ICouncilRegistry {
    function isCouncil(address who) external view returns (bool);
    function councilCount() external view returns (uint256);
}

interface ICouncilEndorsements {
    function endorsedCount(uint256 proposalId) external view returns (uint256);
}

/// @notice Execution layer controlled by Council. Multisig-ready: requires a minimum endorsement threshold before execution.
/// Threshold is controlled by PressParameters: keccak256("council_execute_min_bps") where 3500 = 35%.
contract CouncilExecutor {
    address public councilRegistry;
    address public proposalCenter;
    address public parameters;
    address public endorsements;

    bytes32 public constant P_EXEC_MIN_BPS = keccak256("council_execute_min_bps");

    event Executed(uint256 indexed proposalId, bytes32 indexed key, uint256 value, address indexed by);
    event EndorsementsSet(address endorsements);

    modifier onlyCouncil() {
        require(ICouncilRegistry(councilRegistry).isCouncil(msg.sender), "COUNCIL_ONLY");
        _;
    }

    constructor(address _councilRegistry, address _proposalCenter) {
        councilRegistry = _councilRegistry;
        proposalCenter = _proposalCenter;
    }

    function setParameters(address p) external onlyCouncil {
        parameters = p;
    }

    function setEndorsements(address e) external onlyCouncil {
        endorsements = e;
        emit EndorsementsSet(e);
    }

    function _meetsEndorsementThreshold(uint256 proposalId) internal view returns (bool) {
        if(endorsements == address(0)) return true; // allow if not configured (dev)
        uint256 cnt = ICouncilEndorsements(endorsements).endorsedCount(proposalId);
        uint256 total = ICouncilRegistry(councilRegistry).councilCount();
        if(total == 0) return false;
        uint256 minBps = IPressParameters(parameters).params(P_EXEC_MIN_BPS);
        if(minBps == 0) minBps = 3500;
        // ceil(total*minBps/10000)
        uint256 need = (total * minBps + 9999) / 10000;
        if(need < 1) need = 1;
        return cnt >= need;
    }

    /// payload format for PARAM_CHANGE:
    /// abi.encode(bytes32 key, uint256 value)
    function executeParamChange(uint256 proposalId) external onlyCouncil {
        require(IProposalCenter(proposalCenter).proposalPassed(proposalId), "NOT_PASSED");
        require(!IProposalCenter(proposalCenter).proposalExecuted(proposalId), "ALREADY_EXECUTED");
        require(_meetsEndorsementThreshold(proposalId), "NOT_ENOUGH_ENDORSEMENTS");

        bytes memory payload = IProposalCenter(proposalCenter).getProposalPayload(proposalId);
        (bytes32 key, uint256 value) = abi.decode(payload, (bytes32, uint256));
        IPressParameters(parameters).set(key, value);

        // lock execution
        IProposalCenter(proposalCenter).markExecuted(proposalId);

        emit Executed(proposalId, key, value, msg.sender);
    }
}
