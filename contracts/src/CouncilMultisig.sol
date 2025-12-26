// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

/**
 * Minimal multisig intended for Council execution of approved parameter changes.
 * Owners capped at 195. Threshold must be >= ceil(35% of owners) to match policy intent.
 */
contract CouncilMultisig {
    uint256 public constant MAX_OWNERS = 195;

    mapping(address => bool) public isOwner;
    address[] public owners;
    uint256 public threshold;

    struct Tx {
        address target;
        uint256 value;
        bytes data;
        bool executed;
        uint256 approvals;
    }

    mapping(uint256 => Tx) public txs;
    mapping(uint256 => mapping(address => bool)) public approvedBy;
    uint256 public txCount;

    event OwnerAdded(address indexed owner);
    event OwnerRemoved(address indexed owner);
    event OwnerListSynced(uint256 ownerCount);
    event ThresholdSet(uint256 threshold);
    event TxProposed(uint256 indexed txId, address indexed target, uint256 value);
    event TxApproved(uint256 indexed txId, address indexed by);
    event TxExecuted(uint256 indexed txId);

    modifier onlyOwnerOrSelf() {
        require(isOwner[msg.sender] || msg.sender == address(this), "OWNER_ONLY");
        _;
    }

    modifier onlyOwner() {
        require(isOwner[msg.sender], "OWNER_ONLY");
        _;
    }

    constructor(address[] memory _owners, uint256 _threshold) {
        require(_owners.length > 0 && _owners.length <= MAX_OWNERS, "OWNERS");
        for (uint256 i=0;i<_owners.length;i++) {
            address o = _owners[i];
            require(o != address(0) && !isOwner[o], "OWNER_DUP");
            isOwner[o]=true;
            owners.push(o);
            emit OwnerAdded(o);
        }
        _setThreshold(_threshold);
    }

    function ownerCount() external view returns (uint256) { return owners.length; }

    function minPolicyThreshold() public view returns (uint256) {
        // ceil(35% of owners)
        return (owners.length * 35 + 99) / 100;
    }

    function _setThreshold(uint256 t) internal {
        uint256 minT = minPolicyThreshold();
        require(t >= minT, "THRESHOLD_POLICY");
        require(t <= owners.length, "THRESHOLD_MAX");
        threshold = t;
        emit ThresholdSet(t);
    }

    function setThreshold(uint256 t) external onlyOwnerOrSelf { _setThreshold(t); }

function addOwner(address o) external onlyOwnerOrSelf {
    require(o != address(0) && !isOwner[o], "OWNER");
    require(owners.length + 1 <= MAX_OWNERS, "MAX_195");
    owners.push(o);
    isOwner[o] = true;
    // Ensure existing threshold still meets new 35% policy
    require(threshold >= minPolicyThreshold(), "THRESHOLD_POLICY");
    emit OwnerAdded(o);
    emit OwnerListSynced(owners.length);
}

function removeOwner(address o) external onlyOwnerOrSelf {
    require(isOwner[o], "NOT_OWNER");
    // remove
    isOwner[o] = false;
    for (uint256 i=0;i<owners.length;i++) {
        if (owners[i] == o) {
            owners[i] = owners[owners.length-1];
            owners.pop();
            break;
        }
    }
    // adjust threshold if needed
    uint256 minT = minPolicyThreshold();
    if (threshold < minT) {
        threshold = minT;
        emit ThresholdSet(threshold);
    }
    require(threshold <= owners.length, "THRESHOLD_MAX");
    emit OwnerRemoved(o);
    emit OwnerListSynced(owners.length);
}

    function propose(address target, uint256 value, bytes calldata data) external onlyOwner returns (uint256) {
        txCount++;
        uint256 id = txCount;
        txs[id] = Tx({target: target, value: value, data: data, executed: false, approvals: 0});
        emit TxProposed(id, target, value);
        approve(id);
        return id;
    }

    function approve(uint256 id) public onlyOwner {
        Tx storage t = txs[id];
        require(!t.executed, "EXECUTED");
        require(!approvedBy[id][msg.sender], "ALREADY");
        approvedBy[id][msg.sender] = true;
        t.approvals += 1;
        emit TxApproved(id, msg.sender);
        if (t.approvals >= threshold) {
            _execute(id);
        }
    }

    function execute(uint256 id) external onlyOwner { _execute(id); }

    function _execute(uint256 id) internal {
        Tx storage t = txs[id];
        require(!t.executed, "EXECUTED");
        require(t.approvals >= threshold, "APPROVALS");
        t.executed = true;
        (bool ok,) = t.target.call{value: t.value}(t.data);
        require(ok, "CALL_FAIL");
        emit TxExecuted(id);
    }
}
