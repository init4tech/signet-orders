// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {RollupOrders} from "zenith/src/orders/RollupOrders.sol";

import {SignetStd} from "./SignetStd.sol";

/// @title GetOut
/// @author init4
/// @notice A contract that gets out of the Rollup by converting native USD to
///         to USDC on the host network.
/// @dev This contract inherits the SignetStd contract and automatically
///      configures rollup constants on construction.
contract GetOut is SignetStd() {
    /// @notice Thrown when no value is sent to the contract.
    error NoValue();

    /// @notice Simply forwards any native asset sent to this contract
    ///         to the `getOut` function.
    receive() external payable {
        getOut();
    }

    /// @notice Converts native USD to USDC on the host network with a flat
    ///         50bps fee.
    /// @custom:reverts NoValue when no value is sent to the contract.
    /// @custom:emits OrderOrigin.Order
    function getOut() public payable {
        require(msg.value > 0, NoValue());

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
