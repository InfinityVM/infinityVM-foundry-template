// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {JobManager} from "../src/coprocessor/JobManager.sol";
import {IJobManager} from "../src/coprocessor/IJobManager.sol";
import {Consumer} from "../src/coprocessor/Consumer.sol";
import {SquareRootConsumer} from "../src/SquareRootConsumer.sol";
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
    SquareRootConsumer public consumer;

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

        consumer = new SquareRootConsumer(address(jobManager));

        // Set ELF paths
        jobManager.setElfPath(
            bytes32(0xc90c53832dd8f018556a7d74f6eb0be5bd32ab9cee2b13659193ec38bcf6f2d7),
            "/Users/zeke/ethos/code/infinity-foundry-template/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/square-root"
        );

        vm.stopBroadcast();
    }
}
