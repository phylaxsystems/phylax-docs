// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Reject a zero debt amount with outstanding debt shares at transaction end.
contract DebtShareConsistency is Assertion {
    constructor() { registerAssertionSpec(AssertionSpec.Reshiram); }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        (uint128 elastic, uint128 base) = abi.decode(read(ph.getAssertionAdopter(),
            abi.encodeWithSignature("totalBorrow()"), _postTx()), (uint128, uint128));
        require(elastic != 0 || base == 0, "Debt shares without debt");
    }

    function read(address target, bytes memory data, PhEvm.ForkId memory fork)
        internal view returns (bytes memory)
    {
        PhEvm.StaticCallResult memory result = ph.staticcallAt(target, data, 500_000, fork);
        require(result.ok, "Snapshot read failed");
        return result.data;
    }
}
