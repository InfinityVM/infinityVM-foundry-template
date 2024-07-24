// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {JobManager} from "../src/JobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {ExampleConsumer} from "../src/ExampleConsumer.sol";
import {CoprocessorDeployer} from "../script/CoprocessorDeployer.s.sol";

contract CoprocessorTest is Test, CoprocessorDeployer {
    uint64 DEFAULT_MAX_CYCLES = 1_000_000;
    address RELAYER = address(1);
    address COPROCESSOR_OPERATOR = 0x184c47137933253f49325B851307Ab1017863BD0;

    event JobCreated(uint32 indexed jobID, uint64 maxCycles, bytes32 indexed programID, bytes programInput);
    event JobCancelled(uint32 indexed jobID);
    event JobCompleted(uint32 indexed jobID, bytes result);

    function setUp() public {
        deployCoprocessorContracts(RELAYER, COPROCESSOR_OPERATOR);
    }

    function test_Consumer_RequestJob() public {
        uint32 jobID = consumer.requestBalance(address(2));
    }
}
