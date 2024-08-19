// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {IJobManager, JOB_STATE_PENDING, JOB_STATE_CANCELLED, JOB_STATE_COMPLETED} from "./IJobManager.sol";
import {Consumer} from "./Consumer.sol";
import {OffchainRequester} from "./OffchainRequester.sol";
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import {Initializable} from "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";
import "./Utils.sol";
import {Script, console} from "forge-std/Script.sol";
import {Test} from "forge-std/Test.sol";
import {StdCheatsSafe} from "forge-std/StdCheats.sol";
import {CommonBase} from "forge-std/Base.sol";
import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";
import {ECDSA} from "solady/utils/ECDSA.sol";

contract JobManager is 
    IJobManager,
    Initializable,
    OwnableUpgradeable, 
    ReentrancyGuard,
    CommonBase
{
    using Strings for uint;
    using Strings for uint32;
    using Strings for uint64;
    using Strings for address;
    using Utils for bytes;
    using Utils for bytes32;

    // bytes4(keccak256("isValidSignature(bytes32,bytes)")
    bytes4 constant internal EIP1271_MAGIC_VALUE = 0x1626ba7e;

    address public relayer;
    // This operator is a registered entity that will eventually require some bond from participants
    address public coprocessorOperator;

    // Mapping from job ID --> job metadata
    mapping(bytes32 => JobMetadata) public jobIDToMetadata;
    // Mapping from program ID (verification key) --> ELF path
    mapping(bytes32 => string) public programIDToElfPath;
    // storage gap for upgradeability
    uint256[50] private __GAP;

    constructor() {
        _disableInitializers();
    }

    function initializeJobManager(address initialOwner, address _relayer, address _coprocessorOperator) public initializer {
        _transferOwnership(initialOwner);
        relayer = _relayer;
        coprocessorOperator = _coprocessorOperator;
    }

    function getRelayer() external view returns (address) {
        return relayer;
    }


    function getCoprocessorOperator() external view returns (address) {
        return coprocessorOperator;
    }

    function getJobMetadata(bytes32 jobID) public view returns (JobMetadata memory) {
        return jobIDToMetadata[jobID];
    }

    function getElfPath(bytes32 programID) public view returns (string memory) {
        return programIDToElfPath[programID];
    }

    function setRelayer(address _relayer) external onlyOwner {
        relayer = _relayer;
    }

    function setCoprocessorOperator(address _coprocessorOperator) external onlyOwner {
        coprocessorOperator = _coprocessorOperator;
    }

    function setElfPath(bytes32 programID, string calldata elfPath) external onlyOwner {
        programIDToElfPath[programID] = elfPath;
    }

    function createJob(uint64 nonce, bytes32 programID, bytes calldata programInput, uint64 maxCycles) external override returns (bytes32) {
        address consumer = msg.sender;
        bytes32 jobID = keccak256(abi.encodePacked(nonce, consumer));
       _createJob(nonce, jobID, programID, maxCycles, consumer);
        emit JobCreated(jobID, maxCycles, programID, programInput);

        string memory elfPath = getElfPath(programID);
        // This would normally be a separate call by relayer, but for tests we call it here
        (bytes memory resultWithMetadata, bytes memory signature) = execute(elfPath, programInput, jobID, maxCycles);
        submitResult(resultWithMetadata, signature);

        return jobID;
    }

    function _createJob(uint64 nonce, bytes32 jobID, bytes32 programID, uint64 maxCycles, address consumer) internal {
        require(jobIDToMetadata[jobID].status == 0, "JobManager.createJob: job already exists with this nonce and consumer");
        jobIDToMetadata[jobID] = JobMetadata(programID, maxCycles, consumer, JOB_STATE_PENDING);
        Consumer(consumer).updateLatestNonce(nonce);
    }

    function requestOffchainJob(bytes32 programID, bytes calldata input, uint64 maxCycles, address consumer, uint64 nonce, string calldata privateKey) public {
        (bytes memory resultWithMetadata, bytes memory resultSignature, bytes memory jobRequest, bytes memory requestSignature) = executeOffchainJob(programID, input, maxCycles, consumer, nonce, privateKey);

        submitResultForOffchainJob(resultWithMetadata, resultSignature, jobRequest, requestSignature);
    }

    // CancelJob is not useful in the current Foundry template since createJob calls submitResult directly,
    // so there's no way to cancel a job before it's completed.
    function cancelJob(bytes32 jobID) external override {
        JobMetadata memory job = jobIDToMetadata[jobID];
        // We allow the JobManager owner to also cancel jobs so Ethos admin can veto any jobs
        require(msg.sender == job.caller || msg.sender == owner(), "JobManager.cancelJob: caller is not the job creator or JobManager owner");

        require(job.status == JOB_STATE_PENDING, "JobManager.cancelJob: job is not in pending state");
        job.status = JOB_STATE_CANCELLED;
        jobIDToMetadata[jobID] = job;

        emit JobCancelled(jobID);
    }

    function submitResult(
        bytes memory resultWithMetadata, // Includes job ID + program input hash + max cycles + program ID + result value
        bytes memory signature
    ) public override nonReentrant {
        // Verify signature on result with metadata
        bytes32 messageHash = ECDSA.toEthSignedMessageHash(resultWithMetadata);
        require(ECDSA.tryRecover(messageHash, signature) == coprocessorOperator, "JobManager.submitResult: Invalid signature");

        // Decode the resultWithMetadata using abi.decode
        ResultWithMetadata memory result = decodeResultWithMetadata(resultWithMetadata);

        _submitResult(result.jobID, result.maxCycles, result.programInputHash, result.programID, result.result);
    }

    function submitResultForOffchainJob(
        bytes memory offchainResultWithMetadata,
        bytes memory signatureOnResult,
        bytes memory jobRequest,
        bytes memory signatureOnRequest
    ) public override {
        // Decode the job request using abi.decode
        OffchainJobRequest memory request = decodeJobRequest(jobRequest);

        bytes32 jobID = keccak256(abi.encodePacked(request.nonce, request.consumer));

        // Verify signature on job request
        bytes32 requestHash = ECDSA.toEthSignedMessageHash(jobRequest);
        require(OffchainRequester(request.consumer).isValidSignature(requestHash, signatureOnRequest) == EIP1271_MAGIC_VALUE, "JobManager.submitResultForOffchainJob: Invalid signature on job request");

        // Verify signature on result with metadata
        bytes32 resultHash = ECDSA.toEthSignedMessageHash(offchainResultWithMetadata);
        require(ECDSA.tryRecover(resultHash, signatureOnResult) == coprocessorOperator, "JobManager.submitResultForOffchainJob: Invalid signature on result");

        // Create a job and set program inputs on consumer
        _createJob(request.nonce, jobID, request.programID, request.maxCycles, request.consumer);

        // Decode the result using abi.decode
        OffChainResultWithMetadata memory result = decodeOffchainResultWithMetadata(offchainResultWithMetadata);
        _submitResult(jobID, result.maxCycles, result.programInputHash, result.programID, result.result);
    }

    function _submitResult(
        bytes32 jobID,
        uint64 maxCycles,
        bytes32 programInputHash,
        bytes32 programID,
        bytes memory result
    ) internal {
        JobMetadata memory job = jobIDToMetadata[jobID];
        require(job.status == JOB_STATE_PENDING, "JobManager.submitResult: job is not in pending state");

        // This prevents the coprocessor from using arbitrary inputs to produce a malicious result
        require(keccak256(Consumer(job.caller).getProgramInputsForJob(jobID)) == programInputHash, 
            "JobManager.submitResult: program input signed by coprocessor doesn't match program input submitted with job");
        
        // This is to prevent coprocessor from using a different program ID to produce a malicious result
        require(job.programID == programID,
            "JobManager.submitResult: program ID signed by coprocessor doesn't match program ID submitted with job");
        
        require(job.maxCycles == maxCycles, "JobManager.submitResult: max cycles signed by coprocessor doesn't match max cycles submitted with job");

        // Update job status to COMPLETED
        job.status = JOB_STATE_COMPLETED;
        jobIDToMetadata[jobID] = job;
        emit JobCompleted(jobID, result);

        // Forward result to consumer
        Consumer(job.caller).receiveResult(jobID, result);
    }

    function execute(string memory elfPath, bytes memory input, bytes32 jobID, uint64 maxCycles) internal returns (bytes memory, bytes memory) {
        string[] memory imageRunnerInput = new string[](12);
        uint256 i = 0;
        imageRunnerInput[i++] = "cargo";
        imageRunnerInput[i++] = "run";
        imageRunnerInput[i++] = "--manifest-path";
        imageRunnerInput[i++] = "zkvm-utils/Cargo.toml";
        imageRunnerInput[i++] = "--bin";
        imageRunnerInput[i++] = "zkvm-utils";
        imageRunnerInput[i++] = "-q";
        imageRunnerInput[i++] = "execute";
        imageRunnerInput[i++] = elfPath;
        imageRunnerInput[i++] = input.toHexString();
        imageRunnerInput[i++] = jobID.toHexString();
        imageRunnerInput[i++] = maxCycles.toString();
        return abi.decode(vm.ffi(imageRunnerInput), (bytes, bytes));
    }

    function executeOffchainJob(bytes32 programID, bytes calldata input, uint64 maxCycles, address consumer, uint64 nonce, string calldata privateKey) internal returns (bytes memory, bytes memory, bytes memory, bytes memory) {
        string memory elfPath = getElfPath(programID);
        string[] memory imageRunnerInput = new string[](14);
        uint256 i = 0;
        imageRunnerInput[i++] = "cargo";
        imageRunnerInput[i++] = "run";
        imageRunnerInput[i++] = "--manifest-path";
        imageRunnerInput[i++] = "zkvm-utils/Cargo.toml";
        imageRunnerInput[i++] = "--bin";
        imageRunnerInput[i++] = "zkvm-utils";
        imageRunnerInput[i++] = "-q";
        imageRunnerInput[i++] = "execute-offchain-job";
        imageRunnerInput[i++] = elfPath;
        imageRunnerInput[i++] = input.toHexString();
        imageRunnerInput[i++] = maxCycles.toString();
        imageRunnerInput[i++] = consumer.toHexString();
        imageRunnerInput[i++] = nonce.toString();
        imageRunnerInput[i++] = privateKey;
        return abi.decode(vm.ffi(imageRunnerInput), (bytes, bytes, bytes, bytes));
    }

    function decodeResultWithMetadata(bytes memory resultWithMetadata) public pure returns (ResultWithMetadata memory) {
        (bytes32 jobID, bytes32 programInputHash, uint64 maxCycles, bytes32 programID, bytes memory result) = abi.decode(resultWithMetadata, (bytes32, bytes32, uint64, bytes32, bytes));
        return ResultWithMetadata(jobID, programInputHash, maxCycles, programID, result);
    }

    function decodeOffchainResultWithMetadata(bytes memory offChainResultWithMetadata) public pure returns (OffChainResultWithMetadata memory) {
        (bytes32 programInputHash, uint64 maxCycles, bytes32 programID, bytes memory result) = abi.decode(offChainResultWithMetadata, (bytes32, uint64, bytes32, bytes));
        return OffChainResultWithMetadata(programInputHash, maxCycles, programID, result);
    }

    function decodeJobRequest(bytes memory jobRequest) public pure returns (OffchainJobRequest memory) {
        (uint64 nonce, uint64 maxCycles, address consumer, bytes32 programID, bytes memory programInput) = abi.decode(jobRequest, (uint64, uint64, address, bytes32, bytes));
        return OffchainJobRequest(nonce, maxCycles, consumer, programID, programInput);
    }

}
