// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Check the donating subaccount before liquidation can hide the unsafe transition.
contract EulerDonationHealth is Assertion {
    address public immutable exec;
    bytes4 constant DONATE = bytes4(keccak256("donateToReserves(uint256,uint256)"));
    constructor(address exec_) {
        registerAssertionSpec(AssertionSpec.Reshiram);
        require(exec_ != address(0), "Invalid exec"); exec = exec_;
    }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        PhEvm.CallInputs[] memory calls = ph.getCallInputs(ph.getAssertionAdopter(), DONATE);
        for (uint256 i; i < calls.length; ++i) {
            (uint256 subAccountId,) = abi.decode(calls[i].input, (uint256, uint256));
            require(subAccountId < 256, "Invalid subaccount");
            address account = address(uint160(calls[i].caller) ^ uint160(subAccountId));
            (uint256 collateral, uint256 liability,,) = abi.decode(read(exec,
                abi.encodeWithSignature("liquidity(address)", account), _postCall(calls[i].id)),
                (uint256, uint256, uint256, bool));
            require(collateral >= liability, "Donation leaves unsafe debt");
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
