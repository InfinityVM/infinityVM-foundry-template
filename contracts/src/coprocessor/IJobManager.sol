// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

// CONSTANTS
uint8 constant JOB_STATE_PENDING = 1; // We start from 1 to avoid the default value of 0 for an empty item in a mapping in Solidity
uint8 constant JOB_STATE_CANCELLED = 2;
uint8 constant JOB_STATE_COMPLETED = 3;

interface IJobManager {
    // EVENTS
    event JobCreated(bytes32 indexed jobID, uint64 indexed nonce, address indexed consumer, uint64 maxCycles, bytes32 programID, bytes onchainInput);
    event JobCancelled(bytes32 indexed jobID);
    event JobCompleted(bytes32 indexed jobID, bytes result);

    // STRUCTS
    struct JobMetadata {
        bytes32 programID;
        uint64 maxCycles;
        address consumer;
        uint8 status;
    }

    struct ResultWithMetadata {
        bytes32 jobID;
        bytes32 onchainInputHash;
        uint64 maxCycles;
        bytes32 programID;
        bytes result;
    }

    struct OffchainResultWithMetadata {
        bytes32 jobID;
        bytes32 onchainInputHash;
        bytes32 offchainInputHash;
        uint64 maxCycles;
        bytes32 programID;
        bytes result;
        bytes32[] versionedBlobHashes;
    }

    struct OffchainJobRequest {
        uint64 nonce;
        uint64 maxCycles;
        address consumer;
        bytes32 programID;
        bytes onchainInput;
        bytes32 offchainInputHash;
    }

    // FUNCTIONS
    function createJob(uint64 nonce, bytes32 programID, bytes calldata onchainInput, uint64 maxCycles) external returns (bytes32 jobID);
    function getJobMetadata(bytes32 jobID) external view returns (JobMetadata memory);
    function cancelJob(bytes32 jobID) external;
    function submitResult(bytes calldata resultWithMetadata, bytes calldata signature) external;
    function submitResultForOffchainJob(bytes calldata offchainResultWithMetadata, bytes calldata signatureOnResult, bytes calldata jobRequest, bytes calldata signatureOnRequest) external;
    function requestOffchainJob(OffchainJobRequest memory request, bytes calldata offchainInput, string calldata privateKey) external;
    function setRelayer(address _relayer) external;
    function getRelayer() external view returns (address);
    function setCoprocessorOperator(address _coprocessorOperator) external;
    function getCoprocessorOperator() external view returns (address);
}
