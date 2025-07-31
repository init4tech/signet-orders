// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {RollupOrders} from "zenith/src/orders/RollupOrders.sol";
import {RollupPassage} from "zenith/src/passage/RollupPassage.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";


/// @title PecorinoConstants
/// @author init4
/// @notice Constants for the Pecorino testnet.
/// @dev These constants are used to configure the SignetStd contract in its
///      constructor, if the chain ID matches the Pecorino testnet chain ID.
library PecorinoConstants {
    /// @notice The Pecorino host chain ID.
    uint32 constant HOST_CHAIN_ID = 3151908;
    /// @notice The Pecorino Rollup chain ID.
    uint32 constant ROLLUP_CHAIN_ID = 14174;

    /// @notice The Rollup Passage contract for the Pecorino testnet.
    RollupPassage constant PECORINO_ROLLUP_PASSAGE = RollupPassage(payable(0x0000000000007369676E65742D70617373616765));

    /// @notice The Rollup Orders contract for the Pecorino testnet.
    RollupOrders constant PECORINO_ROLLUP_ORDERS = RollupOrders(0x000000000000007369676E65742D6f7264657273);

    /// USDC token for the Pecorino testnet host chain.
    address constant HOST_USDC = 0x885F8DB528dC8a38aA3DDad9D3F619746B4a6A81;
    /// USDT token for the Pecorino testnet host chain.
    address constant HOST_USDT = 0x7970D259D4a96764Fa9B23FF0715A35f06f52D1A;
    /// WBTC token for the Pecorino testnet host chain.
    address constant HOST_WBTC = 0x9aeDED4224f3dD31aD8A0B1FcD05E2d7829283a7;
    /// WETH token for the Pecorino testnet host chain.
    address constant HOST_WETH = 0x572C4d72080ed9E9997509b583a22B785B70cB3f;

    /// @notice WETH token address for the Pecorino testnet.
    IERC20 constant WETH = IERC20(0x0000000000000000007369676e65742d77657468);
    /// @notice WBTC token address for the Pecorino testnet.
    IERC20 constant WBTC = IERC20(0x0000000000000000007369676e65742D77627463);
    /// @notice WUSD token address for the Pecorino testnet.
    IERC20 constant WUSD = IERC20(0x0000000000000000007369676e65742D77757364);

}

contract SignetStd {
    /// @notice The native asset address, used as a sentinel for native USD on
    ///         the rollup, or native ETH on the host.
    address constant NATIVE_ASSET = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;

    /// @notice The chain ID of the host network.
    uint32 internal immutable HOST_CHAIN_ID;

    /// @notice The Rollup Passage contract.
    RollupPassage internal immutable PASSAGE;
    /// @notice The Rollup Orders contract.
    RollupOrders internal immutable ORDERS;

    /// @notice The WETH token address.
    IERC20  internal immutable WETH;
    /// @notice The WBTC token address.
    IERC20  internal immutable WBTC;
    /// @notice The WUSD token address.
    IERC20  internal immutable WUSD;

    /// @notice The USDC token address on the host network.
    address internal immutable HOST_USDC;
    /// @notice The USDT token address on the host network.
    address internal immutable HOST_USDT;
    /// @notice The WBTC token address on the host network.
    address internal immutable HOST_WBTC;
    /// @notice The WETH token address on the host network.
    address internal immutable HOST_WETH;

    constructor () {
        // Auto-configure based on the chain ID.
        if (block.chainid == PecorinoConstants.ROLLUP_CHAIN_ID) {
            HOST_CHAIN_ID = PecorinoConstants.HOST_CHAIN_ID;

            PASSAGE = PecorinoConstants.PECORINO_ROLLUP_PASSAGE;
            ORDERS = PecorinoConstants.PECORINO_ROLLUP_ORDERS;

            WETH = PecorinoConstants.WETH;
            WBTC = PecorinoConstants.WBTC;
            WUSD = PecorinoConstants.WUSD;

            HOST_USDC = PecorinoConstants.HOST_USDC;
            HOST_USDT = PecorinoConstants.HOST_USDT;
            HOST_WBTC = PecorinoConstants.HOST_WBTC;
            HOST_WETH = PecorinoConstants.HOST_WETH;
        } else {
            revert("Unsupported chain");
        }
    }
}