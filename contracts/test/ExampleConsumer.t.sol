// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {JobManager} from "../src/JobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {ExampleConsumer} from "../src/ExampleConsumer.sol";
import {Deployer} from "../script/Deployer.s.sol";
import {ProgramID} from "../src/ProgramID.sol";

contract ExampleConsumerTest is Test, Deployer {
    uint64 DEFAULT_MAX_CYCLES = 1_000_000;
    address RELAYER = address(1);
    address COPROCESSOR_OPERATOR = 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266;

    function setUp() public {
        deployContracts(RELAYER, COPROCESSOR_OPERATOR);
    }

    function test_Consumer_RequestJob() public {
        uint32 jobID = consumer.requestSquareRoot(9);
        
        assertEq(jobID, 1);
        assertEq(consumer.getProgramInputsForJob(jobID), abi.encode(9));
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(jobID);
        assertEq(jobMetadata.programID, ProgramID.SQUARE_ROOT_ID);
        assertEq(jobMetadata.maxCycles, DEFAULT_MAX_CYCLES);
        assertEq(jobMetadata.caller, address(consumer));

        // Job status is COMPLETED since createJob in JobManager calls
        // submitResult in this Foundry template
        assertEq(jobMetadata.status, 3);

        // Check that state was correctly updated in Consumer contract
        assertEq(consumer.getSquareRoot(9), 3);
        assertEq(consumer.getJobResult(1), abi.encode(9, 3));
    }

    function testRevertWhen_Consumer_ReceiveResultUnauthorized() public {
        test_Consumer_RequestJob();
        vm.prank(address(1));
        vm.expectRevert("Consumer.onlyJobManager: caller is not the job manager");
        consumer.receiveResult(1, abi.encode(9, 4));
    }
}
