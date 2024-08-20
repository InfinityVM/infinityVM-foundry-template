// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;
import {JobManager} from "./coprocessor/JobManager.sol";
import {Consumer} from "./coprocessor/Consumer.sol";
import {OffchainRequester} from "./coprocessor/OffchainRequester.sol";
import {ProgramID} from "./ProgramID.sol"; 
import {console} from "forge-std/Script.sol";
import {ECDSA} from "solady/utils/ECDSA.sol";

contract SquareRootConsumer is Consumer, OffchainRequester {
    address private offchainSigner;

    mapping(uint256 => uint256) public numberToSquareRoot;
    mapping(bytes32 => bytes) public jobIDToResult;

    uint64 public constant DEFAULT_MAX_CYCLES = 1_000_000;

    constructor(address jobManager, address _offchainSigner, uint64 initialMaxNonce) Consumer(jobManager, initialMaxNonce) OffchainRequester() {
        // SquareRootConsumer allows a single offchainSigner address to sign all offchain job requests
        offchainSigner = _offchainSigner;
    }

    function getSquareRoot(uint256 number) public view returns (uint256) {
        return numberToSquareRoot[number];
    }

    function getJobResult(bytes32 jobID) public view returns (bytes memory) {
        return jobIDToResult[jobID];
    }

    function getOffchainSigner() external view returns (address) {
        return offchainSigner;
    }

    function requestSquareRoot(uint256 number) public returns (bytes32) {
        return requestJob(ProgramID.SQUARE_ROOT_ID, abi.encode(number), DEFAULT_MAX_CYCLES);
    }

    function _receiveResult(bytes32 jobID, bytes memory result) internal override {
        // Decode the coprocessor result into AddressWithBalance
        (uint256 originalNumber, uint256 squareRoot) = abi.decode(result, (uint256, uint256));

        // Perform app-specific logic using the result
        numberToSquareRoot[originalNumber] = squareRoot;
        jobIDToResult[jobID] = result;
    }

    // Included for EIP-1271. The JobManager calls this function to verify the signature on
    // an offchain job request.
    function isValidSignature(bytes32 messageHash, bytes memory signature) public view override returns (bytes4) {
        address recoveredSigner = ECDSA.tryRecover(messageHash, signature);
        // SquareRootConsumer allows a single offchainSigner address to sign all offchain job requests
        if (recoveredSigner == offchainSigner) {
            return EIP1271_MAGIC_VALUE;
        } else {
            return INVALID_SIGNATURE;
        }
    }
}
