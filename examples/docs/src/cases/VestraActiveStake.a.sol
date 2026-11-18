// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Check the stake immediately before each unStake(uint8) call.
contract VestraActiveStake is Assertion {
    bytes4 constant UNSTAKE = bytes4(keccak256("unStake(uint8)"));
    constructor() { registerAssertionSpec(AssertionSpec.Reshiram); }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        address staking = ph.getAssertionAdopter();
        PhEvm.CallInputs[] memory calls = ph.getCallInputs(staking, UNSTAKE);
        for (uint256 i; i < calls.length; ++i) {
            uint8 maturity = abi.decode(calls[i].input, (uint8)); // selector already removed
            (,,,,, bool active) = abi.decode(read(staking,
                abi.encodeWithSignature("stakes(address,uint8)", calls[i].caller, maturity),
                _preCall(calls[i].id)), (uint64, uint256, uint256, uint256, uint64, bool));
            require(active, "Stake already redeemed");
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
