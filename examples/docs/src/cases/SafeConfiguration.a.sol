// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Enforce a minimum threshold and approved owner set at transaction end.
contract SafeConfiguration is Assertion {
    uint256 public immutable minimum;
    mapping(address => bool) public allowed;
    constructor(uint256 minimum_, address[] memory owners) {
        registerAssertionSpec(AssertionSpec.Reshiram);
        require(minimum_ > 0 && minimum_ <= owners.length, "Invalid minimum");
        minimum = minimum_;
        for (uint256 i; i < owners.length; ++i) {
            require(owners[i] != address(0) && !allowed[owners[i]], "Invalid owner");
            allowed[owners[i]] = true;
        }
    }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        address safe = ph.getAssertionAdopter();
        uint256 threshold = abi.decode(read(safe, abi.encodeWithSignature("getThreshold()"), _postTx()), (uint256));
        address[] memory owners = abi.decode(read(safe, abi.encodeWithSignature("getOwners()"), _postTx()), (address[]));
        require(threshold >= minimum && threshold <= owners.length, "Unsafe threshold");
        for (uint256 i; i < owners.length; ++i) require(allowed[owners[i]], "Unapproved owner");
    }

    function read(address target, bytes memory data, PhEvm.ForkId memory fork)
        internal view returns (bytes memory)
    {
        PhEvm.StaticCallResult memory result = ph.staticcallAt(target, data, 500_000, fork);
        require(result.ok, "Snapshot read failed");
        return result.data;
    }
}
