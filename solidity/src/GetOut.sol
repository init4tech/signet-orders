// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {RollupOrders} from "zenith/src/orders/RollupOrders.sol";

import {SignetStd} from "./SignetStd.sol";

contract GetOut is SignetStd() {

    fallback() external payable {
        getOut();
    }

    receive() external payable {
        getOut();
    }

    function getOut() public payable {
        uint256 desired = msg.value * 995 / 1000; // 0.5% fee

        RollupOrders.Input[] memory inputs = new RollupOrders.Input[](1);
        inputs[0].token = NATIVE_ASSET;
        inputs[0].amount = msg.value;

        RollupOrders.Output[] memory outputs = new RollupOrders.Output[](1);
        outputs[0].token = HOST_USDC;
        outputs[0].amount = desired;
        outputs[0].recipient = msg.sender;
        outputs[0].chainId = 1; // Mainnet chain ID

        ORDERS.initiate{value: msg.value}(
            block.timestamp, // deadline
            inputs,
            outputs
        );
    }
}
