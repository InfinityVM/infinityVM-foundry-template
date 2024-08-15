Infinity CLOB PoC

TODO:

- [x] state persistence/DB impl (zeke)
- [ ] endpoints for viewing current orderbook state
- [ ] e2e test/demo of just http based flows
 (zeke)
- [ ] protocol spec
- [ ] architecture diagram
- [ ] modify zkvm program to batch process
- [ ] batcher logic to create batches. Initially this should just submit all batches to CN
- [ ] event listener logic for listening to withdraw / deposits
- [ ] smart contracts
   - [ ] withdraw / deposit
   - [ ] filled orders

- [ ] if the general structure of this server and fraud proofing makes sense, we can make it into an sdk that primarily has zkvm STF, execution engine, and batcher.

## Docs

Terms:

- Global Index: each state modifying action gets a unique global index. This index is incremented by 1 with each mutating action.
- Order ID: each order in the order book gets unique order ID.
- CLOB State: includes latest order ID, the fill status of each order, each users balances, and the orderbook.
- Orderbook: where active orders live.
- tick: the state transition function (STF) for the clob. A request and the current state go into the tick and the response and new state is returned.
- clob program: the zkvm program that runs the STF.

### External

Intuition:

- each request goes through the tick STF.
- requests along with state can be replayed through the clob program to verify any single state transition.
- we can leverage InfinityVM to verify requests.

A naive solution would be to have a batch relayer tack batches of requests and send the batches as a offchain request to CN. The results would then be posted on chain. To improve this from an efficiency perspective we can instead:

- have a batcher service create request batches
- take the state at the end of any request batch and publish its hash on chain
- if fraud is detected:
    - someone can post the previous state and the request batch to the CN. 
    - the CN will publish the results onchain
    - the logic accepting the results can compare the submitted state with the state hash the clob originally submitted
    - if they differ, the clob operator gets slashed

The one issue here is that the global index cannot be validated. To remedy this, all requests will be signed with their global index (`SIGN(request||global_index)`). 
- If anyone can submit two signed requests with the same index, the clob operator gets slashed
- TBD: how to prove a certain index was skipped?

### Internal

#### General Flow

- http endpoint gets request
- http handler request is sent of chanel to engine
- engine assigns request global index
- engine records request in db, keyed by global index
- engine runs tick over request and state
- engine records response and state in db, keyed by global index
- engine sends response and global index over one shot channel back to handler
- handler returns response and global index back to user

## Running

```
cargo run --bin node
```