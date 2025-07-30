// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {RollupOrders} from "zenith/src/orders/RollupOrders.sol";
import {RollupPassage} from "zenith/src/passage/RollupPassage.sol";
import {IERC20} from "openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";


library PecorinoConstants {
    RollupPassage constant PECORINO_ROLLUP_PASSAGE = RollupPassage(payable(0x0000000000007369676E65742D70617373616765));

    RollupOrders constant PECORINO_ROLLUP_ORDERS = RollupOrders(0x000000000000007369676E65742D6f7264657273);

    /// USDC token for the Pecorino testnet host chain.
    address constant HOST_USDC = 0x885F8DB528dC8a38aA3DDad9D3F619746B4a6A81;
    /// USDT token for the Pecorino testnet host chain.
    address constant HOST_USDT = 0x7970D259D4a96764Fa9B23FF0715A35f06f52D1A;
    /// WBTC token for the Pecorino testnet host chain.
    address constant HOST_WBTC = 0x9aeDED4224f3dD31aD8A0B1FcD05E2d7829283a7;
    /// WETH token for the Pecorino testnet host chain.
    address constant HOST_WETH = 0x572C4d72080ed9E9997509b583a22B785B70cB3f;

    IERC20 constant WETH = IERC20(0x0000000000000000007369676e65742d77657468);
    IERC20 constant WBTC = IERC20(0x0000000000000000007369676e65742D77627463);
    IERC20 constant WUSD = IERC20(0x0000000000000000007369676e65742D77757364);
}

contract SignetStd {
    address constant NATIVE_ASSET = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;

    RollupPassage internal immutable PASSAGE;
    RollupOrders internal immutable ORDERS;

    IERC20  internal immutable WETH;
    IERC20  internal immutable WBTC;
    IERC20  internal immutable WUSD;

    address internal immutable HOST_USDC;
    address internal immutable HOST_USDT;
    address internal immutable HOST_WBTC;
    address internal immutable HOST_WETH;

    constructor () {
        if (block.chainid == 14174) {
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