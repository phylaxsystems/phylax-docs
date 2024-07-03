pragma solidity ^0.8.0;

import {Alert} from "phylax-std/Alert.sol";

contract StorageAlert is Alert {
    uint256 testStorage;

    function testStoragePersistence() public {
        require(testStorage == 0, "Storage should be 0");
        testStorage = 1;
        require(testStorage == 1, "Storage should be 1");
    }
}
