// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "./JobManager.sol";
import {console} from "forge-std/Script.sol";

abstract contract Consumer {
    JobManager internal _jobManager;
    mapping(uint32 => bytes) internal jobIDToProgramInput;

    constructor(address __jobManager) {
        _jobManager = JobManager(__jobManager);
    }

    modifier onlyJobManager() {
        require(
            msg.sender == address(_jobManager),
            "Consumer.onlyJobManager: caller is not the job manager"
        );
        _;
    }

    function requestJob(
        bytes32 programID,
        bytes memory programInput,
        uint64 maxCycles
    ) internal returns (uint32) {
        uint32 jobID = _jobManager.createJob(programID, programInput, maxCycles);
        jobIDToProgramInput[jobID] = programInput;
        return jobID;
    }

    // CancelJob is not useful in the current Foundry template since createJob in the JobManager
    // calls submitResult directly, so there's no way to cancel a job before it's completed.
    function cancelJob(uint32 jobID) internal {
        _jobManager.cancelJob(jobID);
    }

    function getProgramInputsForJob(uint32 jobID) public view returns (bytes memory) {
        return jobIDToProgramInput[jobID];
    }

    function receiveResult(uint32 jobID, bytes calldata result) external onlyJobManager {
        _receiveResult(jobID, result);
    }

    // This function must be overridden by the app-specific Consumer contract
    // to decode the coprocessor result into any app-specific struct and
    // perform app-specific logic using the result
    function _receiveResult(uint32 jobID, bytes memory result) internal virtual;
}
