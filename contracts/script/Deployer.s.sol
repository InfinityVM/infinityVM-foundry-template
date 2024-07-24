// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {JobManager} from "../src/JobManager.sol";
import {IJobManager} from "../src/IJobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {ExampleConsumer} from "../src/ExampleConsumer.sol";
import {Utils} from "./utils/Utils.sol";
import "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import "./utils/EmptyContract.sol";

// To deploy and verify:
// forge script Deployer.s.sol:Deployer --sig "deployContracts(address relayer, address coprocessorOperator)" $RELAYER $COPROCESSOR_OPERATOR --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract Deployer is Script, Utils {
    ProxyAdmin public coprocessorProxyAdmin;
    JobManager public jobManager;
    IJobManager public jobManagerImplementation;
    ExampleConsumer public consumer;

    function deployContracts(address relayer, address coprocessorOperator) public {
        vm.startBroadcast();
        // deploy proxy admin for ability to upgrade proxy contracts
        coprocessorProxyAdmin = new ProxyAdmin();

        jobManagerImplementation = new JobManager();
        jobManager = JobManager(
            address(
                new TransparentUpgradeableProxy(
                    address(jobManagerImplementation),
                    address(coprocessorProxyAdmin),
                    abi.encodeWithSelector(
                        jobManager.initializeJobManager.selector, msg.sender, relayer, coprocessorOperator
                    )
                )
            )
        );

        consumer = new ExampleConsumer(address(jobManager));

        // Set ELF paths
        jobManager.setElfPath(
            bytes32(0xf413ab25c9ce7fe3e85448336135f0885c1cab35df0c444e11550f504d8f22ff),
            "/Users/maanavkhaitan/Desktop/mesh-security-avs/infinity-foundry-template/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/address-balance"
        );

        vm.stopBroadcast();
    }
}