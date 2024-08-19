// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import {Consumer} from "../src/coprocessor/Consumer.sol";
import {SquareRootConsumer} from "../src/SquareRootConsumer.sol";
import {Deployer} from "../script/Deployer.s.sol";
import {ProgramID} from "../src/ProgramID.sol";

contract SquareRootConsumerTest is Test, Deployer {
    uint64 DEFAULT_MAX_CYCLES = 1_000_000;
    address RELAYER = address(1);
    address COPROCESSOR_OPERATOR = 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266;
    address OFFCHAIN_REQUEST_SIGNER = 0xaF6Bcd673C742723391086C1e91f0B29141D2381;
    string DEFAULT_OFFCHAIN_SIGNER_PRIVATE_KEY = "0x0c7ec7aefb80022c0025be1e72dadb0679aa294cb1db453b2e7b5da8616b4e31";
    uint64 DEFAULT_INITIAL_MAX_NONCE = 0;
    uint64 DEFAULT_NONCE = 1;
    bytes32 DEFAULT_JOB_ID;

    function setUp() public {
        deployContracts(RELAYER, COPROCESSOR_OPERATOR, OFFCHAIN_REQUEST_SIGNER, DEFAULT_INITIAL_MAX_NONCE);
        DEFAULT_JOB_ID = keccak256(abi.encodePacked(DEFAULT_NONCE, address(consumer)));
    }

    function test_Consumer_RequestJob() public {
        bytes32 jobID = consumer.requestSquareRoot(9);
        
        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(jobID);
        assertEq(jobMetadata.programID, ProgramID.SQUARE_ROOT_ID);

        // Job status is COMPLETED since createJob in JobManager calls
        // submitResult in this Foundry template
        assertEq(jobMetadata.status, 3);

        // Check that state was correctly updated in Consumer contract
        assertEq(consumer.getSquareRoot(9), 3);
        assertEq(consumer.getJobResult(DEFAULT_JOB_ID), abi.encode(9, 3));
    }

    function test_Consumer_RequestOffchainJob() public {
        // Request offchain job from default offchain user
        jobManager.requestOffchainJob(
            ProgramID.SQUARE_ROOT_ID, // Program ID
            abi.encode(9), // Program input
            DEFAULT_MAX_CYCLES, // Max cycles
            address(consumer), // Consumer address to send result to
            DEFAULT_NONCE, // Nonce (should be unique for each offchain job request)
            DEFAULT_OFFCHAIN_SIGNER_PRIVATE_KEY // Private key of offchain request signer
        );

        JobManager.JobMetadata memory jobMetadata = jobManager.getJobMetadata(DEFAULT_JOB_ID);
        assertEq(jobMetadata.programID, ProgramID.SQUARE_ROOT_ID);

        // Job status is COMPLETED since createJob in JobManager calls
        // submitResult in this Foundry template
        assertEq(jobMetadata.status, 3);

        // Check that state was correctly updated in Consumer contract
        assertEq(consumer.getSquareRoot(9), 3);
        assertEq(consumer.getJobResult(DEFAULT_JOB_ID), abi.encode(9, 3));

        // Check inputs are set correctly in consumer
        assertEq(consumer.getProgramInputsForJob(DEFAULT_JOB_ID), abi.encode(9));

        // Check that nonce-related data is stored correctly in Consumer contract
        assertEq(consumer.getNextNonce(), 2);
    }

    function testRevertWhen_Consumer_ReceiveResultUnauthorized() public {
        test_Consumer_RequestJob();
        vm.prank(address(1));
        vm.expectRevert("Consumer.onlyJobManager: caller is not the job manager");
        consumer.receiveResult(DEFAULT_JOB_ID, abi.encode(9, 4));
    }
}
