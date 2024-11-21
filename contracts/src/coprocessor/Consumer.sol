// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "./JobManager.sol";
import {console} from "forge-std/Script.sol";

abstract contract Consumer {
    struct JobInputs {
        bytes onchainInput;
        bytes32 offchainInputHash;
    }

    JobManager internal _jobManager;
    uint64 public maxNonce;

    mapping(bytes32 => JobInputs) internal jobIDToInputs;

    constructor(address __jobManager, uint64 _initialMaxNonce) {
        _jobManager = JobManager(__jobManager);
        maxNonce = _initialMaxNonce;
    }

    modifier onlyJobManager() {
        require(
            msg.sender == address(_jobManager),
            "Consumer.onlyJobManager: caller is not the job manager"
        );
        _;
    }

    function getOnchainInputForJob(bytes32 jobID) public view virtual returns (bytes memory) {
        return jobIDToInputs[jobID].onchainInput;
    }

    function getOffchainInputHashForJob(bytes32 jobID) public view virtual returns (bytes32) {
        return jobIDToInputs[jobID].offchainInputHash;
    }

    // Returns the next nonce to be used for a job
    function getNextNonce() public view virtual returns (uint64) {
        return maxNonce + 1;
    }

    // We have a single setter function here to improve gas efficiency
    function setInputsForJob(
        bytes32 jobID,
        bytes memory onchainInput,
        bytes32 offchainInputHash
    ) public virtual onlyJobManager() {
        JobInputs storage jobInputs = jobIDToInputs[jobID];
        jobInputs.onchainInput = onchainInput;
        jobInputs.offchainInputHash = offchainInputHash;
    }

    // Updates the maxNonce if the latest nonce is greater than the current maxNonce
    function updateLatestNonce(uint64 nonce) public virtual onlyJobManager() {
        if (nonce > maxNonce) {
            maxNonce = nonce;
        }
    }

    function requestJob(
        bytes memory programID,
        bytes memory onchainInput,
        uint64 maxCycles
    ) internal virtual returns (bytes32) {
        bytes32 jobID = _jobManager.createJob(getNextNonce(), programID, onchainInput, maxCycles);
        // Set onchain input
        jobIDToInputs[jobID].onchainInput = onchainInput;
        return jobID;
    }

    // CancelJob is not useful in the current Foundry template since createJob in the JobManager
    // calls submitResult directly, so there's no way to cancel a job before it's completed.
    function cancelJob(bytes32 jobID) internal virtual {
        _jobManager.cancelJob(jobID);
    }

    function receiveResult(bytes32 jobID, bytes calldata result) external onlyJobManager() {
        _receiveResult(jobID, result);
    }

    // This function must be overridden by the app-specific Consumer contract
    // to decode the coprocessor result into any app-specific struct and
    // perform app-specific logic using the result
    function _receiveResult(bytes32 jobID, bytes memory result) internal virtual;
}
