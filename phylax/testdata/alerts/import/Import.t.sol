pragma solidity ^0.8.0;
import {Alert} from "phylax-std/Alert.sol";
import {console} from "forge-std/console.sol";

contract ImportAlert is Alert {
    uint256 $ethereum;

    function setUp() public {
        $ethereum = enableChain("ethereum", 19999);
    }

    function testBlockNumber() public chain($ethereum) {
        assertEq(block.number, 19999);
    }

    function testImport() public chain($ethereum) {
        string memory taskName = ph.importContextString("activity.task_name");
        uint256 eventOrigin = ph.importContextUint("event.origin");
        assertEq(taskName, "test");
        assertEq(eventOrigin, 123);
    }
}
