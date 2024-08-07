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
    using Utils for bytes;

    // bytes4(keccak256("isValidSignature(bytes32,bytes)")
    bytes4 constant internal EIP1271_MAGIC_VALUE = 0x1626ba7e;

    uint32 internal jobIDCounter;
    address public relayer;
    // This operator is a registered entity that will eventually require some bond from participants
    address public coprocessorOperator;

    mapping(uint32 => JobMetadata) public jobIDToMetadata;
    // We store nonceHashToJobID to prevent replay attacks by the coprocessor of a user's job request
    mapping(bytes32 => uint32) public nonceHashToJobID;
    // We store consumerToMaxNonce to help consumers keep track of the maximum nonce they have used so far
    mapping(address => uint64) public consumerToMaxNonce;
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
        jobIDCounter = 1;
    }

    function getRelayer() external view returns (address) {
        return relayer;
    }


    function getCoprocessorOperator() external view returns (address) {
        return coprocessorOperator;
    }

    function getJobMetadata(uint32 jobID) public view returns (JobMetadata memory) {
        return jobIDToMetadata[jobID];
    }

    function getJobIDForNonce(uint64 nonce, address consumer) public view returns (uint32) {
        bytes32 nonceHash = keccak256(abi.encodePacked(nonce, consumer));
        return nonceHashToJobID[nonceHash];
    }

    function getMaxNonce(address consumer) public view returns (uint64) {
        return consumerToMaxNonce[consumer];
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

    function createJob(bytes32 programID, bytes memory programInput, uint64 maxCycles) external override returns (uint32) {
        uint32 jobID = _createJob(programID, programInput, maxCycles, msg.sender);

        string memory elfPath = getElfPath(programID);
        // This would normally be a separate call by relayer, but for tests we call it here
        (bytes memory resultWithMetadata, bytes memory signature) = execute(elfPath, programInput, jobID, maxCycles);
        submitResult(resultWithMetadata, signature);

        return jobID;
    }

    function _createJob(bytes32 programID, bytes memory programInput, uint64 maxCycles, address consumer) internal returns (uint32) {
        uint32 jobID = jobIDCounter;
        jobIDToMetadata[jobID] = JobMetadata(programID, maxCycles, consumer, JOB_STATE_PENDING);
        emit JobCreated(jobID, maxCycles, programID, programInput);
        jobIDCounter++;

        Consumer(consumer).setProgramInputsForJob(jobID, programInput);

        return jobID;
    }

    // CancelJob is not useful in the current Foundry template since createJob calls submitResult directly,
    // so there's no way to cancel a job before it's completed.
    function cancelJob(uint32 jobID) external override {
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
        bytes calldata offchainResultWithMetadata,
        bytes calldata signatureOnResult,
        bytes calldata jobRequest,
        bytes calldata signatureOnRequest
    ) public override returns (uint32) {
        // Decode the job request using abi.decode
        OffchainJobRequest memory request = decodeJobRequest(jobRequest);

        // Check if nonce already exists
        bytes32 nonceHash = keccak256(abi.encodePacked(request.nonce, request.consumer));
        require(nonceHashToJobID[nonceHash] == 0, "JobManager.submitResultForOffchainJob: Nonce already exists for this consumer");

        // Verify signature on job request
        bytes32 requestHash = ECDSA.toEthSignedMessageHash(jobRequest);
        require(OffchainRequester(request.consumer).isValidSignature(requestHash, signatureOnRequest) == EIP1271_MAGIC_VALUE, "JobManager.submitResultForOffchainJob: Invalid signature on job request");

        // Verify signature on result with metadata
        bytes32 resultHash = ECDSA.toEthSignedMessageHash(offchainResultWithMetadata);
        require(ECDSA.tryRecover(resultHash, signatureOnResult) == coprocessorOperator, "JobManager.submitResultForOffchainJob: Invalid signature on result");

        // Create a job and set program inputs on consumer
        uint32 jobID = _createJob(request.programID, request.programInput, request.maxCycles, request.consumer);

        // Update nonce-relevant storage
        nonceHashToJobID[nonceHash] = jobID;
        if (request.nonce > consumerToMaxNonce[request.consumer]) {
            consumerToMaxNonce[request.consumer] = request.nonce;
        }

        // Decode the result using abi.decode
        OffChainResultWithMetadata memory result = decodeOffchainResultWithMetadata(offchainResultWithMetadata);
        _submitResult(jobID, result.maxCycles, result.programInputHash, result.programID, result.result);

        return jobID;
    }

    function _submitResult(
        uint32 jobID,
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

    function execute(string memory elf_path, bytes memory input, uint32 jobID, uint64 maxCycles) internal returns (bytes memory, bytes memory) {
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
        imageRunnerInput[i++] = elf_path;
        imageRunnerInput[i++] = input.toHexString();
        imageRunnerInput[i++] = jobID.toString();
        imageRunnerInput[i++] = maxCycles.toString();
        return abi.decode(vm.ffi(imageRunnerInput), (bytes, bytes));
    }

    function decodeResultWithMetadata(bytes memory resultWithMetadata) public pure returns (ResultWithMetadata memory) {
        (uint32 jobID, bytes32 programInputHash, uint64 maxCycles, bytes32 programID, bytes memory result) = abi.decode(resultWithMetadata, (uint32, bytes32, uint64, bytes32, bytes));
        return ResultWithMetadata(jobID, programInputHash, maxCycles, programID, result);
    }

    function decodeOffchainResultWithMetadata(bytes memory offChainResultWithMetadata) public pure returns (OffChainResultWithMetadata memory) {
        (bytes32 programInputHash, uint64 maxCycles, bytes32 programID, bytes memory result) = abi.decode(offChainResultWithMetadata, (bytes32, uint64, bytes32, bytes));
        return OffChainResultWithMetadata(programInputHash, maxCycles, programID, result);
    }

    function decodeJobRequest(bytes memory jobRequest) public pure returns (OffchainJobRequest memory) {
        (uint64 nonce, uint64 maxCycles, address consumer, bytes32 programID, bytes memory programInput) = abi.decode(jobRequest, (uint32, uint64, address, bytes32, bytes));
        return OffchainJobRequest(nonce, maxCycles, consumer, programID, programInput);
    }

}
