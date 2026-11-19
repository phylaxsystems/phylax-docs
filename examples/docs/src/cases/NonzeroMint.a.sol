// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;

import {Assertion} from "credible-std/Assertion.sol";
import {AssertionSpec} from "credible-std/SpecRecorder.sol";
import {PhEvm} from "credible-std/PhEvm.sol";

/// @notice A positive cToken mint must increase the caller's share balance.
contract NonzeroMint is Assertion {
    bytes4 constant MINT = bytes4(keccak256("mint(uint256)"));
    constructor() { registerAssertionSpec(AssertionSpec.Reshiram); }
    function triggers() external view override { registerTxEndTrigger(this.check.selector); }
    function check() external view {
        address market = ph.getAssertionAdopter();
        PhEvm.CallInputs[] memory calls = ph.getCallInputs(market, MINT);
        for (uint256 i; i < calls.length; ++i) {
            if (abi.decode(calls[i].input, (uint256)) == 0) continue;
            bytes memory query = abi.encodeWithSignature("balanceOf(address)", calls[i].caller);
            uint256 beforeShares = abi.decode(read(market, query, _preCall(calls[i].id)), (uint256));
            uint256 afterShares = abi.decode(read(market, query, _postCall(calls[i].id)), (uint256));
            require(afterShares > beforeShares, "Deposit minted no shares");
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
