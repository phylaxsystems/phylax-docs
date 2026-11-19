// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Reported collateral must not exceed independently computed recoverable value.
/// @dev The reader returns (reportedValue, recoverableValue), both in collateral units.
contract RecoverableCollateral is Assertion {
    address public immutable reader;
    bytes public query;
    bytes4 public immutable operation;
    constructor(address reader_, bytes memory query_, bytes4 operation_) {
        registerAssertionSpec(AssertionSpec.Reshiram);
        require(reader_ != address(0) && query_.length >= 4, "Invalid reader");
        reader = reader_; query = query_; operation = operation_;
    }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        PhEvm.CallInputs[] memory calls = ph.getCallInputs(ph.getAssertionAdopter(), operation);
        for (uint256 i; i < calls.length; ++i) {
            (uint256 reported, uint256 recoverable) = abi.decode(read(reader, query,
                _postCall(calls[i].id)), (uint256, uint256));
            require(reported <= recoverable, "Phantom collateral");
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
