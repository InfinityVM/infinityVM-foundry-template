// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "./coprocessor/JobManager.sol";
import {Consumer} from "./coprocessor/Consumer.sol";
import {SingleOffchainSigner} from "./coprocessor/SingleOffchainSigner.sol";
import {ProgramID} from "./ProgramID.sol"; 
import {console} from "forge-std/Script.sol";
import {ECDSA} from "solady/utils/ECDSA.sol";

contract SquareRootConsumer is Consumer, SingleOffchainSigner {
    mapping(uint256 => uint256) public numberToSquareRoot;
    mapping(bytes32 => bytes) public jobIDToResult;

    uint64 public constant DEFAULT_MAX_CYCLES = 1_000_000;

    constructor(address jobManager, address _offchainSigner, uint64 initialMaxNonce) Consumer(jobManager, initialMaxNonce) SingleOffchainSigner(_offchainSigner) {}

    function getSquareRoot(uint256 number) public view returns (uint256) {
        return numberToSquareRoot[number];
    }

    function getJobResult(bytes32 jobID) public view returns (bytes memory) {
        return jobIDToResult[jobID];
    }

    function requestSquareRoot(uint256 number) public returns (bytes32) {
        return requestJob(ProgramID.SQUARE_ROOT_ID, abi.encode(number), DEFAULT_MAX_CYCLES);
    }

    function _receiveResult(bytes32 jobID, bytes memory result) internal override {
        // Decode the coprocessor result into AddressWithBalance
        (uint256 originalNumber, uint256 squareRoot) = abi.decode(result, (uint256, uint256));

        // Perform app-specific logic using the result
        numberToSquareRoot[originalNumber] = squareRoot;
        jobIDToResult[jobID] = result;
    }
}
