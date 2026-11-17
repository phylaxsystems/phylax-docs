// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;
import {Test} from "forge-std/Test.sol";
import {CredibleTest} from "credible-std/CredibleTest.sol";
import {CallInputsAssertion, LogsAssertion, OracleSnapshotsAssertion} from "../src/ObservationAssertions.a.sol";

contract CounterFixture {
    uint256 public value;
    event Added(uint256 amount);
    event Unrelated(address account);
    function add(uint256 amount) external { value += amount; emit Added(amount); }
    function addTwice(uint256 a, uint256 b) external { this.add(a); this.add(b); }
    function wrongInput(uint256 amount) external { this.add(amount); value++; }
    function wrongLog(uint256 amount) external { value += amount; emit Added(amount + 1); }
    function silent(uint256 amount) external { value += amount; }
    function noise() external { emit Unrelated(msg.sender); }
    function otherEmitter(CounterFixture other) external { other.add(3); }
}
contract OracleFixture {
    uint256 public price;
    constructor(uint256 initial) { price = initial; }
    function updatePrice(uint256 next) external { price = next; }
    function roundTrip(uint256 next) external {
        uint256 initial = price;
        this.updatePrice(next);
        this.updatePrice(initial);
    }
}
contract ObservationAssertionsTest is Test, CredibleTest {
    CounterFixture counter;
    function setUp() public { counter = new CounterFixture(); }
    function inputs() internal { cl.assertion(address(counter), type(CallInputsAssertion).creationCode, CallInputsAssertion.check.selector); }
    function armLogs() internal { cl.assertion(address(counter), type(LogsAssertion).creationCode, LogsAssertion.check.selector); }
    function oracle(OracleFixture target) internal { cl.assertion(address(target), type(OracleSnapshotsAssertion).creationCode, OracleSnapshotsAssertion.check.selector); }
    function testFuzzInputsAllowTwoAdds(uint128 a, uint128 b) public {
        inputs(); counter.addTwice(a, b); assertEq(counter.value(), uint256(a) + b);
    }
    function testInputsRejectUnaccountedWrite() public {
        inputs(); vm.expectRevert(bytes("Input mismatch")); counter.wrongInput(5); assertEq(counter.value(), 0);
    }
    function testInputsAllowUnrelatedCall() public { inputs(); counter.noise(); }
    function testFuzzLogsAllowTwoAdds(uint128 a, uint128 b) public {
        armLogs(); counter.addTwice(a, b); assertEq(counter.value(), uint256(a) + b);
    }
    function testLogsRejectWrongAmount() public { armLogs(); vm.expectRevert(bytes("Log mismatch")); counter.wrongLog(5); }
    function testLogsRejectSilentWrite() public { armLogs(); vm.expectRevert(bytes("Log mismatch")); counter.silent(5); }
    function testLogsIgnoreOtherSignature() public { armLogs(); counter.noise(); }
    function testLogsIgnoreOtherEmitter() public {
        CounterFixture other = new CounterFixture();
        armLogs(); counter.otherEmitter(other);
    }
    function testFuzzOracleInsideBand(uint256 next) public {
        next = bound(next, 90, 110);
        OracleFixture target = new OracleFixture(100); oracle(target); target.updatePrice(next); assertEq(target.price(), next);
    }
    function testFuzzOracleAboveBand(uint256 next) public {
        next = bound(next, 111, type(uint128).max);
        OracleFixture target = new OracleFixture(100); oracle(target);
        vm.expectRevert(bytes("Oracle deviation")); target.updatePrice(next); assertEq(target.price(), 100);
    }
    function testOracleBelowBand() public {
        OracleFixture target = new OracleFixture(100); oracle(target);
        vm.expectRevert(bytes("Oracle deviation")); target.updatePrice(89);
    }
    function testOracleRejectsRestoration() public {
        OracleFixture target = new OracleFixture(100); oracle(target);
        vm.expectRevert(bytes("Oracle deviation")); target.roundTrip(150); assertEq(target.price(), 100);
    }
    function testOracleAllowsRestorationInsideBand() public {
        OracleFixture target = new OracleFixture(100); oracle(target); target.roundTrip(105); assertEq(target.price(), 100);
    }
    function testOracleZeroBaseline() public {
        OracleFixture target = new OracleFixture(0); oracle(target);
        vm.expectRevert(bytes("Oracle deviation")); target.updatePrice(1);
    }
}
