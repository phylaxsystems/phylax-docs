// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice Recheck each cook caller's debt and collateral at the end of that call.
contract CookSolvency is Assertion {
    bytes4 constant COOK = bytes4(keccak256("cook(uint8[],uint256[],bytes[])"));
    constructor() { registerAssertionSpec(AssertionSpec.Reshiram); }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        address cauldron = ph.getAssertionAdopter();
        PhEvm.CallInputs[] memory calls = ph.getCallInputs(cauldron, COOK);
        for (uint256 i; i < calls.length; ++i) {
            checkAccount(cauldron, calls[i].caller, _postCall(calls[i].id));
        }
    }
    function checkAccount(address cauldron, address account, PhEvm.ForkId memory at) internal view {
        uint256 debt = debtOf(cauldron, account, at);
        if (debt == 0) return;
        uint256 amount = collateralOf(cauldron, account, at);
        uint256 rate = number(cauldron, abi.encodeWithSignature("exchangeRate()"), at);
        uint256 ltv = number(cauldron, abi.encodeWithSignature("COLLATERIZATION_RATE()"), at);
        require(rate > 0 && ltv > 0 && ltv <= 100_000, "Invalid risk parameters");
        // Cauldron rate is collateral per MIM, scaled by 1e18. Overflow fails closed.
        require(amount * 1e18 * ltv / 100_000 >= debt * rate, "Insolvent cook caller");
    }
    function debtOf(address cauldron, address account, PhEvm.ForkId memory at) internal view returns (uint256) {
        uint256 part = number(cauldron, abi.encodeWithSignature("userBorrowPart(address)", account), at);
        if (part == 0) return 0;
        (uint128 elastic, uint128 base) = abi.decode(read(cauldron,
            abi.encodeWithSignature("totalBorrow()"), at), (uint128, uint128));
        require(base != 0 && elastic != 0 && part <= base, "Invalid debt accounting");
        return (part * uint256(elastic) + base - 1) / base; // round debt up
    }
    function collateralOf(address cauldron, address account, PhEvm.ForkId memory at) internal view returns (uint256) {
        uint256 collateralShares = number(cauldron,
            abi.encodeWithSignature("userCollateralShare(address)", account), at);
        address bento = abi.decode(read(cauldron, abi.encodeWithSignature("bentoBox()"), at), (address));
        address token = abi.decode(read(cauldron, abi.encodeWithSignature("collateral()"), at), (address));
        return number(bento, abi.encodeWithSignature("toAmount(address,uint256,bool)",
            token, collateralShares, false), at);
    }
    function number(address target, bytes memory query, PhEvm.ForkId memory at) internal view returns (uint256) {
        return abi.decode(read(target, query, at), (uint256));
    }

    function read(address target, bytes memory data, PhEvm.ForkId memory fork)
        internal view returns (bytes memory)
    {
        PhEvm.StaticCallResult memory result = ph.staticcallAt(target, data, 500_000, fork);
        require(result.ok, "Snapshot read failed");
        return result.data;
    }
}
