// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Enforce non-diluting backing on each deposit(uint256,address,address).
contract VisorBacking is Assertion {
    address public immutable asset;
    address public immutable shares;
    bytes4 constant DEPOSIT = bytes4(keccak256("deposit(uint256,address,address)"));
    constructor(address asset_, address shares_) {
        registerAssertionSpec(AssertionSpec.Reshiram);
        require(asset_ != address(0) && shares_ != address(0), "Invalid token");
        asset = asset_; shares = shares_;
    }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        address custody = ph.getAssertionAdopter();
        PhEvm.CallInputs[] memory calls = ph.getCallInputs(custody, DEPOSIT);
        for (uint256 i; i < calls.length; ++i) {
            checkDeposit(custody, _preCall(calls[i].id), _postCall(calls[i].id));
        }
    }
    function checkDeposit(address custody, PhEvm.ForkId memory beforeCall, PhEvm.ForkId memory afterCall) internal view {
        uint256 b0 = balance(custody, beforeCall);
        uint256 b1 = balance(custody, afterCall);
        uint256 s0 = supply(beforeCall);
        uint256 s1 = supply(afterCall);
        if (s1 <= s0) return;
        require(b1 > b0, "Mint without backing");
        // Bounded products fail closed instead of silently overflowing.
        require(b0 <= type(uint128).max && b1 <= type(uint128).max
            && s0 <= type(uint128).max && s1 <= type(uint128).max, "Unsupported magnitude");
        if (s0 != 0) require(b1 * s0 >= b0 * s1, "Share backing diluted");
    }
    function balance(address custody, PhEvm.ForkId memory at) internal view returns (uint256) {
        return abi.decode(read(asset, abi.encodeWithSignature("balanceOf(address)", custody), at), (uint256));
    }
    function supply(PhEvm.ForkId memory at) internal view returns (uint256) {
        return abi.decode(read(shares, abi.encodeWithSignature("totalSupply()"), at), (uint256));
    }

    function read(address target, bytes memory data, PhEvm.ForkId memory fork)
        internal view returns (bytes memory)
    {
        PhEvm.StaticCallResult memory result = ph.staticcallAt(target, data, 500_000, fork);
        require(result.ok, "Snapshot read failed");
        return result.data;
    }
}
