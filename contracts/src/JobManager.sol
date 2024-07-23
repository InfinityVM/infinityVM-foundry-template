// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {IJobManager, JOB_STATE_PENDING, JOB_STATE_CANCELLED, JOB_STATE_COMPLETED} from "./IJobManager.sol";
import {Consumer} from "./Consumer.sol";
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import {Initializable} from "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";
import "./Utils.sol";
import {Script, console} from "forge-std/Script.sol";

contract JobManager is 
    IJobManager,
    Initializable,
    OwnableUpgradeable, 
    ReentrancyGuard
{
    using Utils for uint;

    uint32 internal jobIDCounter;
    address public relayer;
    // This operator is a registered entity that will eventually require some bond from participants
    address public coprocessorOperator;

    mapping(uint32 => JobMetadata) public jobIDToMetadata;
    mapping(bytes32 => string) public imageIDToElfPath;
    // storage gap for upgradeability
    uint256[50] private __GAP;

    constructor() {
        _disableInitializers();
    }

    function initializeJobManager(address initialOwner, address _relayer, address _coprocessorOperator) public initializer {
        _transferOwnership(initialOwner);
        relayer = _relayer;
        coprocessorOperator = _coprocessorOperator;
        jobIDCounter = 1;
    }

    function setRelayer(address _relayer) external onlyOwner {
        relayer = _relayer;
    }

    function getRelayer() external view returns (address) {
        return relayer;
    }

    function setCoprocessorOperator(address _coprocessorOperator) external onlyOwner {
        coprocessorOperator = _coprocessorOperator;
    }

    function getCoprocessorOperator() external view returns (address) {
        return coprocessorOperator;
    }

    function setElfPath(bytes32 imageID, string calldata elfPath) external onlyOwner {
        imageIDToElfPath[imageID] = elfPath;
    }

    function getElfPath(bytes32 imageID) external view returns (string memory) {
        return imageIDToElfPath[imageID];
    }

    function createJob(bytes32 programID, bytes calldata programInput, uint64 maxCycles) external override returns (uint32 jobID) {
        jobID = jobIDCounter;
        jobIDToMetadata[jobID] = JobMetadata(programID, maxCycles, msg.sender, JOB_STATE_PENDING);
        emit JobCreated(jobID, maxCycles, programID, programInput);
        jobIDCounter++;
    }

    function getJobMetadata(uint32 jobID) public view returns (JobMetadata memory) {
        return jobIDToMetadata[jobID];
    }

    function cancelJob(uint32 jobID) external override {
        JobMetadata memory job = jobIDToMetadata[jobID];
        // We allow the JobManager owner to also cancel jobs so Ethos admin can veto any jobs
        require(msg.sender == job.caller || msg.sender == owner(), "JobManager.cancelJob: caller is not the job creator or JobManager owner");

        require(job.status == JOB_STATE_PENDING, "JobManager.cancelJob: job is not in pending state");
        job.status = JOB_STATE_CANCELLED;
        jobIDToMetadata[jobID] = job;

        emit JobCancelled(jobID);
    }

    // This function is called by the relayer
    function submitResult(
        bytes calldata resultWithMetadata, // Includes job ID + program input hash + max cycles + program ID + result value
        bytes calldata signature
    ) external override nonReentrant {
        require(msg.sender == relayer, "JobManager.submitResult: caller is not the relayer");

        // Recover the signer address
        // resultWithMetadata.length needs to be converted to string since the EIP-191 standard requires this 
        bytes32 messageHash = keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n", resultWithMetadata.length.uintToString(), resultWithMetadata));
        address signer = recoverSigner(messageHash, signature);
        require(signer == coprocessorOperator, "JobManager.submitResult: Invalid signature");

        // Decode the resultWithMetadata using abi.decode
        (uint32 jobID, bytes32 programInputHash, uint64 maxCycles, bytes32 programID, bytes memory result) = abi.decode(resultWithMetadata, (uint32, bytes32, uint64, bytes32, bytes));

        JobMetadata memory job = jobIDToMetadata[jobID];
        require(job.status == JOB_STATE_PENDING, "JobManager.submitResult: job is not in pending state");

        // This is to prevent coprocessor from using a different program ID to produce a malicious result
        require(job.programID == programID, 
            "JobManager.submitResult: program ID signed by coprocessor doesn't match program ID submitted with job");

        // This prevents the coprocessor from using arbitrary inputs to produce a malicious result
        require(keccak256(Consumer(job.caller).getProgramInputsForJob(jobID)) == programInputHash, 
            "JobManager.submitResult: program input signed by coprocessor doesn't match program input submitted with job");

        job.status = JOB_STATE_COMPLETED;
        jobIDToMetadata[jobID] = job;

        emit JobCompleted(jobID, result);

        Consumer(job.caller).receiveResult(jobID, result);
    }

    function recoverSigner(bytes32 _ethSignedMessageHash, bytes memory _signature) internal pure returns (address) {
        (bytes32 r, bytes32 s, uint8 v) = splitSignature(_signature);
        return ecrecover(_ethSignedMessageHash, v, r, s);
    }

    function splitSignature(bytes memory sig) internal pure returns (bytes32 r, bytes32 s, uint8 v) {
        require(sig.length == 65, "invalid signature length");

        assembly {
            r := mload(add(sig, 32))
            s := mload(add(sig, 64))
            v := byte(0, mload(add(sig, 96)))
        }

        return (r, s, v);
    }
}
