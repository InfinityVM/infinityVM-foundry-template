Infinity CLOB PoC

## Dev TODO:

Functional PoC:

- [x] state persistence/DB impl (zeke)
- [ ] endpoints for viewing current orderbook state
- [ ] e2e test/demo of just http based flows
- [ ] batcher logic to create batches. Initially this should just submit all batches to CN
 (zeke)
- [ ] modify zkvm program to batch process
- [ ] protocol spec
- [ ] architecture diagram
- [ ] event listener logic for listening to withdraw / deposits
- [ ] smart contracts
   - [ ] withdraw / deposit
   - [ ] filled orders

More production readiness type things:

- [ ] design a system for streaming state out. We may want a secondary node just for state viewing requests (potentially also verifying)
- [ ] verifying deposits
- [ ] verifying txns signed by accounts that have deposit

Future CLOB enhancements:

- [ ] needs test coverage
- [ ] underflow/overflow for arithmetic
- [ ] leverage
- [ ] generalize across pairs - might be big change

- [ ] if the general structure of this server and fraud proofing makes sense, we can make it into an sdk that primarily has zkvm STF, execution engine, and batcher.

Suggestion for stages of refactors for STF verification:

1) we run all request batches through the CN, result in all requests onchain and the end of batch state onchain.
2) State batcher just publishes state checkpoint hash on chain and pushes request batches and state checpoint to DA. We then need fraud proof mode that runs batch through CN and compares resulting state to onchain checkpoint state hash. We also need a way to dispute ordering of requests for this I propose a secondary fraud period where someone can submit request with duplicate global index

## Docs

Terms:

- coprocessor node (CN): infintyVM coprocessor node.
- Global Index: each state modifying action gets a unique global index. This index is incremented by 1 with each mutating action.
- Order ID: each order in the order book gets unique order ID.
- CLOB State: includes latest order ID, the fill status of each order, each users balances, and the orderbook.
- Orderbook: where active orders live. (impl in `core`)
- `tick`: the state transition function (STF) for the clob. A request and the current state go into the tick and the response and new state is returned. (impl in `core`)
- clob program: the zkvm program that runs the STF (tick) in the CN. This program imports the `tick` function and just adds logic to deserialize inputs and re-serialize outputs through zkvm boundaries. (impl in `programs/app`)
- clob node: the service that runs the clob. Responsible for receiving requests, running them through the STF, making state viewable, and batching requests and state.
- node operator: entity responsible for operation of the CLOB node. This entity can be slashed for equivocation in the infinityVM. (impl in `node`)

### External

Intuition:

- Each request goes through the clob node's tick STF.
- Requests along with state can be replayed through the clob program to verify any single state transition.
- We can leverage InfinityVM to verify requests.

A naive solution would be to have a batch relayer tack batches of requests and send the batches as a offchain request to CN. The results would then be posted on chain. To improve this from an efficiency perspective we can instead:

- Have a batcher service create request batches.
- Take the state at the end of any request batch and publish state hash on chain.
- If fraud is detected:
    - Someone can post the previous state and the request batch to the CN. 
    - The CN will publish the results onchain.
    - The logic accepting the results can compare the submitted state with the state hash the clob originally submitted.
    - If they differ, the clob operator gets slashed.

The one issue here is that the global index cannot be validated. To remedy this, all requests will be signed with their global index (`SIGN(request||global_index)`). 
- If anyone can submit two signed requests with the same index, the clob operator gets slashed
- To prove a certain index is skipped one approach is to have the STF validate that each request is sequential to the previous request. We could add the global index to the CLOB State so the STF can ensure it only processes request sequentially. To harden this, we can have the server sign requests, and the STF could verify signatures, making the global index update attributable.

### Internal

#### General Flow

- HTTP endpoint gets request.
- HTTP handler sends request over channel to engine
- Engine assigns request the next global index.
- Engine records request in db, keyed by global index.
- Engine runs tick over request and state.
- Engine records response and new state in db, keyed by global index.
- Engine sends response and global index over one shot channel back to handler
- Handler returns response and global index back to user.

## Running

```
cargo run --bin node
```