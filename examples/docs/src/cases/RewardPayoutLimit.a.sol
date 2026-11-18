// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Cap net COMP paid from a configured rewards treasury in one transaction.
contract RewardPayoutLimit is Assertion {
    address public immutable comp;
    uint256 public immutable maximum;
    constructor(address comp_, uint256 maximum_) {
        registerAssertionSpec(AssertionSpec.Reshiram);
        require(comp_ != address(0), "Invalid COMP"); comp = comp_; maximum = maximum_;
    }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        bytes memory query = abi.encodeWithSignature("balanceOf(address)", ph.getAssertionAdopter());
        uint256 beforeBalance = abi.decode(read(comp, query, _preTx()), (uint256));
        uint256 afterBalance = abi.decode(read(comp, query, _postTx()), (uint256));
        if (beforeBalance > afterBalance) require(beforeBalance - afterBalance <= maximum, "Reward payout above cap");
    }

    function read(address target, bytes memory data, PhEvm.ForkId memory fork)
        internal view returns (bytes memory)
    {
        PhEvm.StaticCallResult memory result = ph.staticcallAt(target, data, 500_000, fork);
        require(result.ok, "Snapshot read failed");
        return result.data;
    }
}
