# InfinityVM Foundry Template

This repository implements an example application using InfinityVM. InfinityVM enables developers to use expressive offchain compute to build and enhance their EVM applications.

## Overview

This repo contains three folders:
1. `programs`: Rust programs that contain application logic to be run offchain in the coprocessor.
2. `contracts`: An `SquareRootConsumer` contract for the application, contracts for the coprocessor, and tests and a deploy script for the contracts.
    - To build on InfinityVM, you just need the `SquareRootConsumer.sol` and `SquareRootConsumer.t.sol` files. The coprocessor contracts in `contracts/coprocessor` expose an interface you can use but you don't need to read how they're implemented.
3. `zkvm-utils`: Utility functions for InfinityVM. *You don't need to read these files to build on InfinityVM.*

The flow of the InfinityVM coprocessor looks like this:
1. An app contract requests a compute job from the coprocessor.
2. The coprocessor picks up this job and submits the result back to the contract.
3. The app contract can simply use the result from the coprocessor in any of their app logic.

![InfinityVM coprocessor flow](image.png)

## Quick Start

This section will take us through an example of building an app that computes and stores the square root of numbers.

First, clone this repo (including submodules):
```
git clone --recursive https://github.com/Ethos-Works/infinity-foundry-template.git
```

### Write a Rust program to run in the coprocessor

All application programs run by the coprocessor live in `programs/app/src`. For our square root application, we have a `square-root.rs` program which takes in an integer and returns the square root. This program is also a good example of how to accept inputs and return output.

This is a simple example but you could write a lot more interesting and complex code in your Rust programs. One thing to note is you can't print anything to `stdout` in your Rust program (if you'd like to print something while debugging your Rust program, we've provided instructions in the `Write tests for your app` section below).

After you've written your Rust program, add it to `programs/app/Cargo.toml`. For example, if we wrote a new program `multiply.rs`, we would add it like this:
```
[package]
name = "guests"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "square-root"
path = "src/square-root.rs"

[[bin]]
name = "multiply"
path = "src/multiply.rs"
```

Now you can run:
```
cargo build
```
This will build your program and update the relevant contracts to allow you to use your program from the contracts. Every program has a unique program ID generated for it, which is added to the `ProgramID.sol` contract.

### Use the program in your app contract

We have a contract for the square root app in `contracts/src/SquareRootConsumer.sol`. To use the `square-root.rs` program, we just need to do two things:

1. Call `requestJob()` with the program ID of `square-root.rs` from `ProgramID.sol` along with ABI-encoded inputs (the number we want to calculate the square root of).
2. Write a `_receiveResult()` function which accepts the output from the `square-root.rs` program and uses it in some application logic.

To build the contracts, you can run:
```
forge build
```

### Write tests for your app

We have an end-to-end test for the `SquareRootConsumer` app in `SquareRootConsumer.t.sol`. This test requests the square root of a number and verifies that the contract calls the `square-root.rs` program and stores the correct result from the coprocessor. You can add any tests for your app contracts in this file.

To run the tests, you can run:
```
forge test -vvv --ffi 
```

If you would like to test or debug your Rust program by itself, we have an example test in `programs/src/lib.rs`. You can run this using:
```
cargo test
```
You can add `println!` statements to your Rust program to help while debugging.

Feel free to reach out to our team if you have any questions, we're happy to help!
