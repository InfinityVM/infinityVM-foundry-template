// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "./JobManager.sol";
import {Consumer} from "./Consumer.sol";
import {ImageID} from "./ImageID.sol"; 

contract ExampleConsumer is Consumer {

    mapping(address => uint256) public addressToBalance;
    mapping(uint32 => bytes) public jobIDToResult;

    uint64 public constant DEFAULT_MAX_CYCLES = 1_000_000;

    constructor(address jobManager) Consumer(jobManager) {}

    function requestBalance(address addr) public returns (uint32) {
        return requestJob(ImageID.ADDRESS_BALANCE_ID, abi.encode(addr), DEFAULT_MAX_CYCLES);
    }

    function getBalance(address addr) public view returns (uint256) {
        return addressToBalance[addr];
    }

    function getJobResult(uint32 jobID) public view returns (bytes memory) {
        return jobIDToResult[jobID];
    }

    function _receiveResult(uint32 jobID, bytes memory result) internal override {
        // Decode the coprocessor result into AddressWithBalance
        (address addr, uint256 balance) = abi.decode(result, (address, uint256));

        // Perform app-specific logic using the result
        addressToBalance[addr] = balance;
        jobIDToResult[jobID] = result;
    }

}
