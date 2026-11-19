// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Bound cumulative rounding error across covered withdrawals.
/// @dev The configured reader returns (shareSupply, active0, active1) for one pool.
contract WithdrawalAccounting is Assertion {
    struct PoolState { uint256 supply; uint256 active0; uint256 active1; }
    address public immutable reader;
    bytes public query;
    bytes4 public immutable withdrawSelector;
    uint256 public immutable roundingBudget;
    constructor(address reader_, bytes memory query_, bytes4 selector_, uint256 budget_) {
        registerAssertionSpec(AssertionSpec.Reshiram);
        require(reader_ != address(0) && query_.length >= 4, "Invalid reader");
        reader = reader_; query = query_; withdrawSelector = selector_; roundingBudget = budget_;
    }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        PhEvm.CallInputs[] memory calls = ph.getCallInputs(ph.getAssertionAdopter(), withdrawSelector);
        uint256 error0; uint256 error1;
        for (uint256 i; i < calls.length; ++i) {
            (uint256 delta0, uint256 delta1) = excess(_preCall(calls[i].id), _postCall(calls[i].id));
            error0 += delta0; error1 += delta1;
        }
        require(error0 <= roundingBudget && error1 <= roundingBudget, "Cumulative withdrawal error");
    }
    function excess(PhEvm.ForkId memory beforeCall, PhEvm.ForkId memory afterCall)
        internal view returns (uint256, uint256)
    {
        PoolState memory a = abi.decode(read(reader, query, beforeCall), (PoolState));
        PoolState memory b = abi.decode(read(reader, query, afterCall), (PoolState));
        require(a.supply > 0 && b.supply <= a.supply && b.active0 <= a.active0
            && b.active1 <= a.active1, "Unexpected withdrawal transition");
        require(a.supply <= type(uint128).max && a.active0 <= type(uint128).max
            && a.active1 <= type(uint128).max, "Unsupported magnitude");
        uint256 burned = a.supply - b.supply;
        uint256 expected0 = a.active0 * burned / a.supply;
        uint256 expected1 = a.active1 * burned / a.supply;
        uint256 removed0 = a.active0 - b.active0;
        uint256 removed1 = a.active1 - b.active1;
        return (removed0 > expected0 ? removed0 - expected0 : 0,
            removed1 > expected1 ? removed1 - expected1 : 0);
    }

    function read(address target, bytes memory data, PhEvm.ForkId memory fork)
        internal view returns (bytes memory)
    {
        PhEvm.StaticCallResult memory result = ph.staticcallAt(target, data, 500_000, fork);
        require(result.ok, "Snapshot read failed");
        return result.data;
    }
}
