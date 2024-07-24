// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {JobManager} from "../src/JobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {ExampleConsumer} from "../src/ExampleConsumer.sol";
import {Deployer} from "../script/Deployer.s.sol";
import {ImageID} from "../src/ImageID.sol";

contract CoprocessorTest is Test, Deployer {
    uint64 DEFAULT_MAX_CYCLES = 1_000_000;
    address RELAYER = address(1);
    address COPROCESSOR_OPERATOR = 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266;

    event JobCreated(uint32 indexed jobID, uint64 maxCycles, bytes32 indexed programID, bytes programInput);
    event JobCompleted(uint32 indexed jobID, bytes result);

    function setUp() public {
        deployContracts(RELAYER, COPROCESSOR_OPERATOR);
    }

    function test_Consumer_RequestJob() public {
        vm.expectEmit(true, true, true, true);
        emit JobCreated(1, DEFAULT_MAX_CYCLES, ImageID.ADDRESS_BALANCE_ID, abi.encode(address(2)));
        uint32 jobID = consumer.requestBalance(address(2));
        assertEq(jobID, 1);
        assertEq(consumer.getProgramInputsForJob(jobID), abi.encode(address(2)));
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(jobID);
        assertEq(jobMetadata.programID, ImageID.ADDRESS_BALANCE_ID);
        assertEq(jobMetadata.maxCycles, DEFAULT_MAX_CYCLES);
        assertEq(jobMetadata.caller, address(consumer));

        // Job status is COMPLETED since createJob in JobManager calls
        // submitResult in this Foundry template
        assertEq(jobMetadata.status, 3);
    }

    function testRevertWhen_Consumer_ReceiveResultUnauthorized() public {
        test_Consumer_RequestJob();
        vm.prank(address(1));
        vm.expectRevert("Consumer.onlyJobManager: caller is not the job manager");
        consumer.receiveResult(1, abi.encode(address(2)));
    }
}
