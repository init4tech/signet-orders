# Signet Orders

This repository provides example code for **placing and filling Orders** on Signet, built using utilities from [signet-sdk](https://github.com/init4tech/signet-sdk).

The repo is intended to _illustrate_ how Fillers can interact with the Signet SDK in order to build Filler software. It is a set of demos and examples; it is not a full-service, out-of-the-box, production-ready Filler. 

---

## Fillers
**Code:** `src/filler.rs`

The `Filler` struct demonstrates the basic steps required to **fill Signet Orders**. Given a set of Orders, it:

1. Constructs and signs **Permit2** structs to fill the Orders on their destination chains.
2. Builds a **Signet Bundle** that batches `initiate` and `fill` transactions for those Orders.
3. Sends the Signet Bundle to the **Transaction Cache**, where it can be mined by Signet Builders.

### Missing Components
In production, a Filler will need to:

- Add **scaffolding** around the the Filler logic to run on a perpetual basis. 
- Implement custom **business logic** to determine which Orders to fill.  
- Potentially extend the example logic with **advanced strategies**, such as performing swaps to source liquidity between fills.

### Filling Strategies
The `Filler` struct demonstrates two strategies for filling Orders:

1. **Fill together (`fill`)**  
   - Submits all Orders/Fills in a single Signet Bundle.  
   - Either **all Orders mine** together or **none do** (atomic execution).

2. **Fill individually (`fill_individually`)**  
   - Submits each Order/Fill in its own Bundle.  
   - Orders succeed or fail **independently**.

**Pros and Cons:**

- **Individual fills** are simpler — Fillers can rely on Builder simulation instead of pre-checking if an Order is already filled. If an `initiate` transaction reverts (because the Order was already filled), the Bundle is simply discarded.
- **Aggregate fills** are **more gas-efficient** and allow strategies like reusing inputs from one Order to fill another. However, if any single Order fails, the **entire Bundle will not mine**.

---

## Orders
**Code:** `src/order.rs`

The `SendOrder` struct provides example code for **initiating an Order**. Given an Order specifying input and output tokens, it:

1. Constructs and signs a **Permit2** struct to initiate the Order on-chain.  
2. Sends the signed Order to the **Transaction Cache**, where Fillers can fill it.

---

## Full Example
**Code:** `bin/roundtrip.rs`

This example executable:

1. Constructs an example Order, signs it, and sends it to the Transaction Cache.  
2. Queries available Orders and fills the example Order.

You can freely modify the example Order to:

- Swap **any set of tokens** on the Host and/or Rollup.  
- Use **multiple Inputs and Outputs**, targeting either the Host or Rollup.

---

### Running the Example

1. **Set environment variables**  
```bash
export CHAIN_NAME=pecorino
export RU_RPC_URL=https://rpc.pecorino.signet.sh/
export HOST_RPC_URL=https://host-rpc.pecorino.signet.sh/
export SIGNER_KEY=[AWS KMS key ID or local private key]
```

2. **Fund your key**  
The example works with **either** an AWS KMS key or a raw local private key.  
This key acts as **both** the Order Initiator and Filler, and must be funded with:

- Input tokens on the Rollup  
- Output tokens on the Host and/or Rollup  
- Gas tokens to pay for Rollup transactions
- Gas tokens to pay for Host transactions

By default, the example swaps **1 Rollup WETH Input → 1 Host WETH Output**, but you can edit the Order freely.

3. **Configure token permissions**  
This example uses **Permit2** for both initiating and filling Orders.  
Approve Permit2 to spend all relevant Input and Output tokens for the key:

```bash
cast send [TOKEN_ADDRESS] "approve(address,uint256)" \
  0x000000000022D473030F116dDEE9F6B43aC78BA3 \
  0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
  --rpc-url $RPC_URL
```

Permit2 uses the same address on Pecorino Rollup and Host as on Ethereum Mainnet.

4. **Run the script**  

This runs an RU to HOST fill.

```bash
cargo run --bin order-roundtrip-example
```

To run an RU to RU fill, pass the `--rollup` flag to the command.

```bash
cargo run --bin order-roundtrip-example -- --rollup    
```

Et voilà! 🎉

---

### Troubleshooting
Signet Bundles target one **specific block number**.
If a Bundle is not included in that exact block, it won't be "retried" in subsequent blocks.

The current example script naively sends each Bundle to one single target block. When running the example, if Bundles are not mining, it is possible that they were simply not included in the target block. A naive solution is to simply re-run the script to try submitting a new Bundle. 

More robust, production-ready software should include bespoke business logic to continually run the Filler logic, such that Bundles are perpetually (re)submitted on a block-by-block basis. 
