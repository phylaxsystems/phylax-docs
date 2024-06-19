pragma solidity ^0.8.0;
import {Alert} from "phylax-std/Alert.sol";

contract FiringAlert is Alert {
    uint256 $ethereum;

    function setUp() public {
        $ethereum = enableChain("ethereum", 19999);
    }

    function testBlockNumber() public chain($ethereum) {
        assertEq(block.number, 9999);
    }

    function testExport() public chain($ethereum) {
        assertEq(block.chainid, 1);
        ph.export("chainid", vm.toString(block.chainid));
        revert();
    }
}
