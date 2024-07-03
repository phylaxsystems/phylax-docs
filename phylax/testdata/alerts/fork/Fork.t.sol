pragma solidity ^0.8.0;

import {Alert} from "phylax-std/Alert.sol";

contract MockContract {
    function call() public {}
}

contract ForkAlert is Alert {
    uint256 ethereum = enableChain("ethereum", 15_000);

    //deploy contract to chain, switch to another chain, call contract, expect revert
    function testSwitchChain() public chain(ethereum) {
        MockContract mock = new MockContract();

        vm.createSelectFork("optimism", 15_000);
        vm.expectRevert();
        mock.call();
    }

    //deploy contract to block, rollback block, call contract, expect revert
    function testSwitchBlock() public chain(ethereum) {
        MockContract mock = new MockContract();

        vm.createSelectFork("ethereum", 10_000);
        vm.expectRevert();
        mock.call();
    }
}
