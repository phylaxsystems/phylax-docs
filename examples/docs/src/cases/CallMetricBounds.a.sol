// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Bound a configured metric before and after every covered operation.
contract CallMetricBounds is Assertion {
    address public immutable reader;
    bytes public query;
    bytes4 public immutable operation;
    uint256 public immutable maximumBps;
    constructor(address reader_, bytes memory query_, bytes4 operation_, uint256 maximumBps_) {
        registerAssertionSpec(AssertionSpec.Reshiram);
        require(reader_ != address(0) && query_.length >= 4, "Invalid reader");
        reader = reader_; query = query_; operation = operation_; maximumBps = maximumBps_;
    }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        address target = ph.getAssertionAdopter();
        uint256 start = abi.decode(read(reader, query, _preTx()), (uint256));
        require(start > 0 && start <= type(uint128).max && maximumBps <= 100_000, "Invalid baseline or bound");
        uint256 allowed = start * maximumBps / 10_000;
        PhEvm.CallInputs[] memory calls = ph.getCallInputs(target, operation);
        for (uint256 i; i < calls.length; ++i) {
            within(start, allowed, _preCall(calls[i].id));
            within(start, allowed, _postCall(calls[i].id));
        }
    }
    function within(uint256 start, uint256 allowed, PhEvm.ForkId memory at) internal view {
        uint256 value = abi.decode(read(reader, query, at), (uint256));
        uint256 delta = value >= start ? value - start : start - value;
        require(delta <= allowed, "Metric outside bound");
    }

    function read(address target, bytes memory data, PhEvm.ForkId memory fork)
        internal view returns (bytes memory)
    {
        PhEvm.StaticCallResult memory result = ph.staticcallAt(target, data, 500_000, fork);
        require(result.ok, "Snapshot read failed");
        return result.data;
    }
}
