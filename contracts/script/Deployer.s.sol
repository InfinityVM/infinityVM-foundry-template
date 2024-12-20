// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

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
// forge script Deployer.s.sol:Deployer --sig "deployContracts(address relayer, address coprocessorOperator, address offchainRequestSigner, uint64 initialMaxNonce)" $RELAYER $COPROCESSOR_OPERATOR $OFFCHAIN_REQUEST_SIGNER $INITIAL_MAX_NONCE --rpc-url $RPC_URL --private-key $PRIVATE_KEY --chain-id $CHAIN_ID --broadcast -v
contract Deployer is Script, Utils {
    ProxyAdmin public coprocessorProxyAdmin;
    JobManager public jobManager;
    IJobManager public jobManagerImplementation;
    SquareRootConsumer public consumer;

    function deployContracts(
        address relayer,
        address coprocessorOperator,
        address offchainRequestSigner,
        uint64 initialMaxNonce
    ) public {
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

        consumer = new SquareRootConsumer(address(jobManager), offchainRequestSigner, initialMaxNonce);

        // Set ELF paths
        jobManager.setElfPath(
            hex"11f33be858c41ce554eea7d643911c8e5c4de4f775da47bf100317333d2ef425", "target/sp1/square-root/square-root"
        );

        vm.stopBroadcast();
    }
}
