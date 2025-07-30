# Signet Orders 

This repository contains example code for placing & filling Orders on Signet. It is built using utilities in [signet-sdk](https://github.com/init4tech/signet-sdk).

## Fillers
Code: `src/filler.rs` 

The `Filler` struct contains example code for the basic steps necessary to Fill Signet Orders. Given a set of Orders as input, the `Filler` struct

- constructs and signs Permit2 struct(s) to Fill the set of Orders on their destination chain(s)
- constructs a Signet Bundle that batches transactions to `intitiate` and `fill` the set of Orders
- sends the Signet Bundle to the Transaction Cache where it can be mined by Signet Builders

#### Missing Components
In a real-life scenario, Fillers will need to implement their own custom business logic to filter which Order(s) they wish to fill. 

Fillers may also wish to modify or extend the simple logic presented here with more complex business logic, such performing swaps to source liquidity in between filling Orders.

#### Filling Strategies

Given a set of Orders to Fill, the `Filler` struct presents two strategies for filling them: 
1. Fill them together (`fill`) - submit all Orders/Fills in one Signet Bundle, such that _all_ of the Orders and Fills mine, or none of them do.
2. Fill them individually (`fill_individually`) - submit each Order/Fill in a separate Signet Bundle, such that they fail or succeed individually. 

A nice feature of filling Orders individually is that Fillers could be less concerned about carefully simulating Orders onchain before attempting to fill them. As long as an Order is economically a "good deal" for the Filler, they can attempt to fill it
without simulating to check whether it has already been filled, because they can rely on Builder simulation. Order `initiate` transactions will revert if the Order has already been filled, in which case the entire Bundle would simply be discarded by the Builder.

On the other hand, Filling Orders in aggregate means that Fills are batched into a single Permit2 and more gas efficient; also, Fillers can use more complex strategies such as utilizing the Inputs of one Order to Fill the subsequent Order. However, if a single Order cannot be filled, then the entire Bundle will not mine. For example, using this strategy, if only one Order is filled by another Filler first, then all other Orders will also not be filled.


## Orders
Code: `src/order.rs`

The `SendOrder` struct contains example code for Initiating an Order. Given an Order struct which specifies the desired Input/Output tokens, the `SendOrder` struct
- constructs and signs Permit2 struct to Initiate the Order on-chain
- sends the Signed Order to the Transaction Cache, where Fillers may fulfill it.

## Full Example 
Code: `bin/roundtrip.rs`

This contains a fully prepared executable binary which (1) constructs an example Order, signs it, and sends it to the Transaction Cache, then (2) queries Orders from the Transaction Cache and Fills the example Order. 

You can edit the example Order to swap any set of tokens on the Host and/or Rollup. Orders can contain multiple Inputs in exchange for multiple Outputs; Outputs can be targeted to the Host or on the Rollup. Feel free to play with it and try it out! 

### Running the Example 

1. Set up the necessary environment variables:
```
export CHAIN_NAME=pecorino
export RU_RPC_URL=https://rpc.pecorino.signet.sh/
export SIGNER_KEY=[AWS KMS key ID or local private key]
export SIGNER_CHAIN_ID=14174
```

2. Fund a key
This example works with _either_ an AWS KMS Key or a raw local private key. This key is both the User that Initiates the Order and the Filler that Fills the Order. As such, it will need to be funded with: 
- the Input tokens on the Rollup
- the Output tokens on the Host and/or Rollup
- gas tokens to pay for transactions on the Rollup

In the default example Order, 1 Rollup WETH Input is swapped for 1 Host WETH Output. Remember you can edit the Order however you want and the example will still work!

3. Configure token permissions 
This example uses Permit2 for Initiating and Filling Orders. As such, one must approve Permit2 to spend all Input and Output tokens from the example key.

You can execute the following command for any relevant token(s):
```
cast send [TOKEN_ADDRESS]  "approve(address,uint256)" 0x000000000022D473030F116dDEE9F6B43aC78BA3 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff --rpc-url $RPC_URL 
```
Note that Permit2 has the same address on Pecorino Rollup and Host chains as it does on Mainnet Ethereum. 

4. Run the script! 
```
cargo run --bin order-roundtrip-examplec
```
Et voilà ~ 

#### Troubleshooting
Signet Bundles are specified to land in one specific block. If you don't see the Bundle included in its specified block for any reason, it will not mine in any subsequent blocks. You can run the script again to retry.