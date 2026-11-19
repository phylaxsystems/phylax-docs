// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Reject any recorded change to the Safe singleton slot, including change-and-restore.
contract SafeImplementation is Assertion {
    constructor() { registerAssertionSpec(AssertionSpec.Reshiram); }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        address safe = ph.getAssertionAdopter();
        bytes32 slot = bytes32(0); // Safe singleton; not the EIP-1967 slot.
        bytes32 initial = ph.loadStateAt(safe, slot, _preTx());
        bytes32[] memory values = ph.getStateChanges(safe, slot);
        for (uint256 i; i < values.length; ++i) {
            require(values[i] == initial, "Safe implementation changed");
        }
    }
}
