// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Freeze the provider's owner, pool admin, and emergency admin at transaction boundaries.
contract RadiantAuthority is Assertion {
    constructor() { registerAssertionSpec(AssertionSpec.Reshiram); }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        address provider = ph.getAssertionAdopter();
        bytes[3] memory getters = [abi.encodeWithSignature("owner()"),
            abi.encodeWithSignature("getPoolAdmin()"), abi.encodeWithSignature("getEmergencyAdmin()")];
        for (uint256 i; i < getters.length; ++i) {
            address beforeRole = abi.decode(read(provider, getters[i], _preTx()), (address));
            address afterRole = abi.decode(read(provider, getters[i], _postTx()), (address));
            require(beforeRole == afterRole, "Authority changed");
        }
    }

    function read(address target, bytes memory data, PhEvm.ForkId memory fork)
        internal view returns (bytes memory)
    {
        PhEvm.StaticCallResult memory result = ph.staticcallAt(target, data, 500_000, fork);
        require(result.ok, "Snapshot read failed");
        return result.data;
    }
}
