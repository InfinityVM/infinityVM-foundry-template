// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {JobManager} from "../src/JobManager.sol";
import {IJobManager} from "../src/IJobManager.sol";
import {Consumer} from "../src/Consumer.sol";
import {MockConsumer} from "../test/mocks/MockConsumer.sol";
import {Utils} from "./utils/Utils.sol";
import "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import "./utils/EmptyContract.sol";

// To deploy and verify:
// forge script CoprocessorDeployer.s.sol:CoprocessorDeployer --sig "deployCoprocessorContracts(address relayer, address coprocessorOperator)" $RELAYER $COPROCESSOR_OPERATOR --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract CoprocessorDeployer is Script, Utils {

    ProxyAdmin public coprocessorProxyAdmin;
    JobManager public jobManager;
    IJobManager public jobManagerImplementation;
    MockConsumer public consumer;

    function deployCoprocessorContracts(address relayer, address coprocessorOperator) public {
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
                        jobManager.initializeJobManager.selector,
                        msg.sender,
                        relayer,
                        coprocessorOperator
                    )
                )
            )
        );

        consumer = new MockConsumer(address(jobManager));
        vm.stopBroadcast();
    }

}
