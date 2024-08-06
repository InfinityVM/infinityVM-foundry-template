// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {IJobManager, JOB_STATE_PENDING, JOB_STATE_CANCELLED, JOB_STATE_COMPLETED} from "./IJobManager.sol";
import {Consumer} from "./Consumer.sol";
import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import {Initializable} from "@openzeppelin-upgrades/contracts/proxy/utils/Initializable.sol";
import "./Utils.sol";
import {Script, console} from "forge-std/Script.sol";
import {Test} from "forge-std/Test.sol";
import {StdCheatsSafe} from "forge-std/StdCheats.sol";
import {CommonBase} from "forge-std/Base.sol";

contract JobManager is 
    IJobManager,
    Initializable,
    OwnableUpgradeable, 
    ReentrancyGuard,
    CommonBase
{
    using Utils for uint;
    using Utils for uint32;
    using Utils for uint64;
    using Utils for bytes;

    uint32 internal jobIDCounter;
    address public relayer;
    // This operator is a registered entity that will eventually require some bond from participants
    address public coprocessorOperator;

    mapping(uint32 => JobMetadata) public jobIDToMetadata;
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

    function setElfPath(bytes32 programID, string calldata elfPath) external onlyOwner {
        programIDToElfPath[programID] = elfPath;
    }

    function getElfPath(bytes32 programID) public view returns (string memory) {
        return programIDToElfPath[programID];
    }

    function createJob(bytes  programID, bytes memory programInput, uint64 maxCycles) external override returns (uint32 jobID) {
        jobID = jobIDCounter;
        jobIDToMetadata[jobID] = JobMetadata(programID, maxCycles, msg.sender, JOB_STATE_PENDING);
        string memory elfPath = getElfPath(programID);
        emit JobCreated(jobID, maxCycles, programID, programInput);
        jobIDCounter++;

        // This would normally be a separate call by relayer, but for tests we call it here
        (bytes memory resultWithMetadata, bytes memory signature) = execute(elfPath, programInput, jobID, maxCycles);
        submitResult(resultWithMetadata, signature);

        return jobID;
    }

    function getJobMetadata(uint32 jobID) public view returns (JobMetadata memory) {
        return jobIDToMetadata[jobID];
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

        job.status = JOB_STATE_COMPLETED;
        jobIDToMetadata[jobID] = job;

        emit JobCompleted(jobID, result);

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
        imageRunnerInput[i++] = jobID.uintToString();
        imageRunnerInput[i++] = maxCycles.uintToString();
        return abi.decode(vm.ffi(imageRunnerInput), (bytes, bytes));
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
