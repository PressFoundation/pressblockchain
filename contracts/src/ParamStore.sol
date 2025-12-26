// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

contract ParamStore {
    address public governance;
    mapping(bytes32 => uint256) public u256;

    event GovernanceSet(address indexed governance);
    event ParamSet(bytes32 indexed key, uint256 value);

    modifier onlyGov() {
        require(msg.sender == governance, "GOV_ONLY");
        _;
    }

    constructor(address gov) {
        governance = gov;
        emit GovernanceSet(gov);
    }

    function setGovernance(address gov) external onlyGov {
        governance = gov;
        emit GovernanceSet(gov);
    }

    function setU256(bytes32 key, uint256 value) external onlyGov {
        u256[key] = value;
        emit ParamSet(key, value);
    }
}
