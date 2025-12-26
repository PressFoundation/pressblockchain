// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice Release batch queue for Press Blockchain.
/// Major upgrades (and optionally other proposals) are queued here once approved.
/// A council multisig (or designated executor) "ships" a batch, emitting explorer-readable events.
interface IPressParameters {
    function get(bytes32 key) external view returns (uint256);
}

contract ReleaseBatchManager {
    event BatchCreated(uint256 indexed batchId, uint256 createdAt, string label, uint256 minReadyAt);
    event ProposalQueued(uint256 indexed batchId, uint256 indexed proposalId, address indexed proposer, uint8 pType);
    event ProposalUnqueued(uint256 indexed batchId, uint256 indexed proposalId, string reason);
    event BatchShipped(uint256 indexed batchId, uint256 shippedAt, string releaseTag, string notesCid);

    IPressParameters public immutable params;
    address public councilExecutor; // expected to be council multisig address in production

    uint256 public batchCount;

    // configurable keys
    bytes32 public constant P_BATCH_MIN_READY_SECONDS = keccak256("release_batch_min_ready_seconds");
    bytes32 public constant P_BATCH_LABEL = keccak256("release_batch_default_label");

    struct Batch {
        uint256 createdAt;
        uint256 minReadyAt;
        string label;
        bool shipped;
        uint256[] proposalIds;
    }

    mapping(uint256 => Batch) public batches;
    mapping(uint256 => uint256) public proposalToBatch; // proposalId -> batchId (0 = none)

    modifier onlyExecutor() {
        require(msg.sender == councilExecutor, "NOT_EXECUTOR");
        _;
    }

    constructor(address pressParameters, address executor) {
        params = IPressParameters(pressParameters);
        councilExecutor = executor;
    }

    function setExecutor(address executor) external onlyExecutor {
        councilExecutor = executor;
    }

    function createBatch(string calldata labelOverride) external onlyExecutor returns (uint256 batchId) {
        batchId = ++batchCount;
        uint256 minReady = params.get(P_BATCH_MIN_READY_SECONDS);
        if(minReady == 0) minReady = 30 days;

        string memory label = labelOverride;
        if(bytes(label).length == 0) {
            // label param is stored as uint; we just default to a simple string
            label = "Monthly Release Batch";
        }

        Batch storage b = batches[batchId];
        b.createdAt = block.timestamp;
        b.minReadyAt = block.timestamp + minReady;
        b.label = label;

        emit BatchCreated(batchId, b.createdAt, b.label, b.minReadyAt);
    }

    function queueProposal(uint256 batchId, uint256 proposalId, address proposer, uint8 pType) external onlyExecutor {
        require(batchId > 0 && batchId <= batchCount, "BAD_BATCH");
        Batch storage b = batches[batchId];
        require(!b.shipped, "BATCH_SHIPPED");
        require(proposalToBatch[proposalId] == 0, "ALREADY_QUEUED");

        b.proposalIds.push(proposalId);
        proposalToBatch[proposalId] = batchId;

        emit ProposalQueued(batchId, proposalId, proposer, pType);
    }

    function unqueueProposal(uint256 proposalId, string calldata reason) external onlyExecutor {
        uint256 batchId = proposalToBatch[proposalId];
        require(batchId != 0, "NOT_QUEUED");
        Batch storage b = batches[batchId];
        require(!b.shipped, "BATCH_SHIPPED");

        // remove from array (swap+pop)
        uint256 len = b.proposalIds.length;
        for(uint256 i=0;i<len;i++){
            if(b.proposalIds[i] == proposalId){
                b.proposalIds[i] = b.proposalIds[len-1];
                b.proposalIds.pop();
                break;
            }
        }
        proposalToBatch[proposalId] = 0;
        emit ProposalUnqueued(batchId, proposalId, reason);
    }

    function shipBatch(uint256 batchId, string calldata releaseTag, string calldata notesCid) external onlyExecutor {
        require(batchId > 0 && batchId <= batchCount, "BAD_BATCH");
        Batch storage b = batches[batchId];
        require(!b.shipped, "ALREADY_SHIPPED");
        require(block.timestamp >= b.minReadyAt, "NOT_READY");

        b.shipped = true;
        emit BatchShipped(batchId, block.timestamp, releaseTag, notesCid);
    }

    function getBatchProposalIds(uint256 batchId) external view returns (uint256[] memory) {
        return batches[batchId].proposalIds;
    }
}
