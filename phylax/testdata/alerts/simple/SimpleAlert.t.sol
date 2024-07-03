pragma solidity ^0.8.0;

import {Alert} from "phylax-std/Alert.sol";

contract SimpleAlert is Alert {
    function setUp() public override {
        super.setUp();
        vm.createSelectFork("ethereum", 0x1a4);
    }

    function testBlockNumber() public {
        ph.export("block.number", vm.toString(block.number));
        assertEq(block.number, 0x1a4);
    }

    function testChainId() public {
        ph.export("chainid", vm.toString(block.chainid));
        assertEq(block.chainid, 1);
    }
}
