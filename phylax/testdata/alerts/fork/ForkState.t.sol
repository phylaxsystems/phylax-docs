pragma solidity ^0.8.0;

import {Alert} from "phylax-std/Alert.sol";

contract ForkStateAlert is Alert {
    uint256 ethereum;

    function setUp() public override {
        super.setUp();
        ethereum = enableChain("ethereum", 5_000_000);
    }

    function testExternalState() public chain(ethereum) {
        IERC20 weth = IERC20(0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2);
        require(weth.decimals() == 18, "weth decimals != 18");
        require(block.number == 5_000_000, "block number != 5_000_000");
    }
}

interface IERC20 {
    function decimals() external view returns (uint256);
}
