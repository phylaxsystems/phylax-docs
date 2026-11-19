// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;
import {Test} from "forge-std/Test.sol";
import {CredibleTest} from "credible-std/CredibleTest.sol";
import {SafeImplementation} from "../src/cases/SafeImplementation.a.sol";
import {DebtShareConsistency} from "../src/cases/DebtShareConsistency.a.sol";
import {CallMetricBounds} from "../src/cases/CallMetricBounds.a.sol";
import {WithdrawalAccounting} from "../src/cases/WithdrawalAccounting.a.sol";
import {RecoverableCollateral} from "../src/cases/RecoverableCollateral.a.sol";
import {VisorBacking} from "../src/cases/VisorBacking.a.sol";
import {NonzeroMint} from "../src/cases/NonzeroMint.a.sol";
import {VestraActiveStake} from "../src/cases/VestraActiveStake.a.sol";

contract SingletonFixture {
    address public singleton = address(0x1234); // slot zero, as in the example's layout
    function keep() external {}
    function change() external { singleton = address(0x5678); }
    function restore() external { address initial = singleton; this.change(); singleton = initial; }
}
contract DebtFixture {
    uint128 public elastic = 100;
    uint128 public base = 100;
    function totalBorrow() external view returns (uint128, uint128) { return (elastic, base); }
    function update(uint128 e, uint128 b) external { elastic = e; base = b; }
}
contract MetricFixture {
    uint256 public metric = 100;
    function update(uint256 next) external { metric = next; }
    function restore(uint256 next) external { this.update(next); this.update(100); }
}
contract WithdrawalFixture {
    uint256 public supply = 100;
    uint256 public active0 = 1000;
    uint256 public active1 = 1000;
    function state() external view returns (uint256, uint256, uint256) { return (supply, active0, active1); }
    function withdraw(uint256 burned, uint256 removed) external {
        supply -= burned; active0 -= removed; active1 -= removed;
    }
    function twoExcessWithdrawals() external { this.withdraw(10, 101); this.withdraw(10, 100); }
}
contract CollateralFixture {
    uint256 reported = 100;
    uint256 recoverable = 100;
    function state() external view returns (uint256, uint256) { return (reported, recoverable); }
    function update(uint256 next) external { reported = next; }
}
contract BackingFixture {
    uint256 public assets = 100;
    uint256 public totalSupply = 100;
    uint256 nextAssets;
    uint256 nextSupply;
    function balanceOf(address) external view returns (uint256) { return assets; }
    function configure(uint256 a, uint256 s) external { nextAssets = a; nextSupply = s; }
    function deposit(uint256, address, address) external { assets = nextAssets; totalSupply = nextSupply; }
}
contract MintFixture {
    mapping(address => uint256) public balanceOf;
    bool public mintShares = true;
    function setMintShares(bool enabled) external { mintShares = enabled; }
    function mint(uint256 amount) external returns (uint256) {
        if (mintShares) balanceOf[msg.sender] += amount;
        return 0;
    }
}
contract StakeFixture {
    mapping(address => bool) active;
    function activate(address account) external { active[account] = true; }
    function stakes(address account, uint8) external view returns (uint64, uint256, uint256, uint256, uint64, bool) {
        return (0, 100, 0, 0, 0, active[account]);
    }
    function unStake(uint8) external { active[msg.sender] = false; } // deliberately missing guard
    function twice() external { this.unStake(0); this.unStake(0); }
}

/// @notice Synthetic checks of example predicates, not historical exploit replays.
contract CaseAssertionsTest is Test, CredibleTest {
    function arm(address target, bytes memory code, bytes4 selector) internal { cl.assertion(target, code, selector); }
    function singleton(SingletonFixture target) internal { arm(address(target), type(SafeImplementation).creationCode, SafeImplementation.check.selector); }
    function testSingletonAllowsUnchanged() public { SingletonFixture t = new SingletonFixture(); singleton(t); t.keep(); }
    function testSingletonRejectsChange() public {
        SingletonFixture t = new SingletonFixture(); singleton(t);
        vm.expectRevert(bytes("Safe implementation changed")); t.change();
    }
    function testSingletonRejectsChangeAndRestore() public {
        SingletonFixture t = new SingletonFixture(); singleton(t);
        vm.expectRevert(bytes("Safe implementation changed")); t.restore();
    }
    function debt(DebtFixture t) internal { arm(address(t), type(DebtShareConsistency).creationCode, DebtShareConsistency.check.selector); }
    function testDebtAllowsFullyRepaid() public { DebtFixture t = new DebtFixture(); debt(t); t.update(0, 0); }
    function testDebtAllowsProportionalReduction() public { DebtFixture t = new DebtFixture(); debt(t); t.update(10, 10); }
    function testDebtRejectsOrphanedShares() public {
        DebtFixture t = new DebtFixture(); debt(t); vm.expectRevert(bytes("Debt shares without debt")); t.update(0, 10);
    }
    function metric(MetricFixture t) internal {
        arm(address(t), abi.encodePacked(type(CallMetricBounds).creationCode,
            abi.encode(address(t), abi.encodeWithSignature("metric()"), MetricFixture.update.selector, uint256(1000))), CallMetricBounds.check.selector);
    }
    function testFuzzMetricAllowsBand(uint256 next) public {
        next = bound(next, 90, 110); MetricFixture t = new MetricFixture(); metric(t); t.update(next);
    }
    function testMetricRejectsHigh() public {
        MetricFixture t = new MetricFixture(); metric(t); vm.expectRevert(bytes("Metric outside bound")); t.update(111);
    }
    function testMetricRejectsLow() public {
        MetricFixture t = new MetricFixture(); metric(t); vm.expectRevert(bytes("Metric outside bound")); t.update(89);
    }
    function testMetricRejectsTemporaryExcursion() public {
        MetricFixture t = new MetricFixture(); metric(t); vm.expectRevert(bytes("Metric outside bound")); t.restore(150);
    }
    function withdrawal(WithdrawalFixture t) internal {
        arm(address(t), abi.encodePacked(type(WithdrawalAccounting).creationCode,
            abi.encode(address(t), abi.encodeWithSignature("state()"), WithdrawalFixture.withdraw.selector, uint256(1))), WithdrawalAccounting.check.selector);
    }
    function testWithdrawalAllowsProportionalExit() public { WithdrawalFixture t = new WithdrawalFixture(); withdrawal(t); t.withdraw(10, 100); }
    function testWithdrawalAllowsOneUnitBudget() public { WithdrawalFixture t = new WithdrawalFixture(); withdrawal(t); t.withdraw(10, 101); }
    function testWithdrawalRejectsAccumulatedError() public {
        WithdrawalFixture t = new WithdrawalFixture(); withdrawal(t);
        vm.expectRevert(bytes("Cumulative withdrawal error")); t.twoExcessWithdrawals();
    }
    function collateral(CollateralFixture t) internal {
        arm(address(t), abi.encodePacked(type(RecoverableCollateral).creationCode,
            abi.encode(address(t), abi.encodeWithSignature("state()"), CollateralFixture.update.selector)), RecoverableCollateral.check.selector);
    }
    function testCollateralAllowsBackedValue() public { CollateralFixture t = new CollateralFixture(); collateral(t); t.update(99); }
    function testCollateralRejectsPhantomValue() public {
        CollateralFixture t = new CollateralFixture(); collateral(t); vm.expectRevert(bytes("Phantom collateral")); t.update(101);
    }
    function backing(BackingFixture t, uint256 a, uint256 s) internal {
        t.configure(a, s);
        arm(address(t), abi.encodePacked(type(VisorBacking).creationCode, abi.encode(address(t), address(t))), VisorBacking.check.selector);
    }
    function testBackingAllowsProportionalMint() public { BackingFixture t = new BackingFixture(); backing(t, 110, 110); t.deposit(10, address(this), address(this)); }
    function testBackingAllowsImprovedRatio() public { BackingFixture t = new BackingFixture(); backing(t, 120, 110); t.deposit(20, address(this), address(this)); }
    function testBackingRejectsDilution() public {
        BackingFixture t = new BackingFixture(); backing(t, 110, 120);
        vm.expectRevert(bytes("Share backing diluted")); t.deposit(10, address(this), address(this));
    }
    function testBackingRejectsUnbackedMint() public {
        BackingFixture t = new BackingFixture(); backing(t, 100, 110);
        vm.expectRevert(bytes("Mint without backing")); t.deposit(0, address(this), address(this));
    }
    function mint(MintFixture t) internal { arm(address(t), type(NonzeroMint).creationCode, NonzeroMint.check.selector); }
    function testMintAllowsNonzeroShares() public { MintFixture t = new MintFixture(); mint(t); t.mint(10); }
    function testMintRejectsZeroShares() public {
        MintFixture t = new MintFixture(); t.setMintShares(false); mint(t);
        vm.expectRevert(bytes("Deposit minted no shares")); t.mint(10);
    }
    function stake(StakeFixture t) internal { arm(address(t), type(VestraActiveStake).creationCode, VestraActiveStake.check.selector); }
    function testStakeAllowsFirstRedemption() public {
        StakeFixture t = new StakeFixture(); t.activate(address(this)); stake(t); t.unStake(0);
    }
    function testStakeRejectsInactive() public {
        StakeFixture t = new StakeFixture(); stake(t); vm.expectRevert(bytes("Stake already redeemed")); t.unStake(0);
    }
    function testStakeRejectsSecondRedemptionInTransaction() public {
        StakeFixture t = new StakeFixture(); t.activate(address(t)); stake(t);
        vm.expectRevert(bytes("Stake already redeemed")); t.twice();
    }
}
