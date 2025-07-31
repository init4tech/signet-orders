

contract PayMe is SignetStd {


    modifier payMe(uint256 amount) {
        RollupOrders.Output[] memory outputs = new RollupOrders.Output[](1);
        outputs[0] = makeRollupOutput(NATIVE_ASSET, amount, address(this), block.chainid);

        ORDERS.initiate{value: amount}(
            type(uint256).max, // this is equivalent to no deadline
            new RollupOrders.Input[](0), // no inputs
            outputs
        );
    }
}