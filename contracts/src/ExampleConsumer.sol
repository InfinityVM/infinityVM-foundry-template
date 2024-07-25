// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "./JobManager.sol";
import {Consumer} from "./Consumer.sol";
import {ImageID} from "./ImageID.sol"; 
import {console} from "forge-std/Script.sol";

contract ExampleConsumer is Consumer {

    mapping(uint256 => uint256) public numberToSquareRoot;
    mapping(uint32 => bytes) public jobIDToResult;

    uint64 public constant DEFAULT_MAX_CYCLES = 1_000_000;

    constructor(address jobManager) Consumer(jobManager) {}

    function requestSquareRoot(uint256 number) public returns (uint32) {
        return requestJob(ImageID.SQUARE_ROOT_ID, abi.encode(number), DEFAULT_MAX_CYCLES);
    }

    function getSquareRoot(uint256 number) public view returns (uint256) {
        return numberToSquareRoot[number];
    }

    function getJobResult(uint32 jobID) public view returns (bytes memory) {
        return jobIDToResult[jobID];
    }

    function _receiveResult(uint32 jobID, bytes memory result) internal override {
        // Decode the coprocessor result into AddressWithBalance
        (uint256 originalNumber, uint256 squareRoot) = abi.decode(result, (uint256, uint256));

        // Perform app-specific logic using the result
        numberToSquareRoot[originalNumber] = squareRoot;
        jobIDToResult[jobID] = result;
    }

}
