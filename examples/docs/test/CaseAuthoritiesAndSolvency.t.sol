// SPDX-License-Identifier: MIT
pragma solidity 0.8.29;
import {Test} from "forge-std/Test.sol";
import {CredibleTest} from "credible-std/CredibleTest.sol";
import {RadiantAuthority} from "../src/cases/RadiantAuthority.a.sol";
import {SafeConfiguration} from "../src/cases/SafeConfiguration.a.sol";
import {CookSolvency} from "../src/cases/CookSolvency.a.sol";
import {EulerDonationHealth} from "../src/cases/EulerDonationHealth.a.sol";
import {RewardPayoutLimit} from "../src/cases/RewardPayoutLimit.a.sol";

contract RolesFixture {
    address public owner = address(0x1234);
    uint256 threshold = 1;
    function getPoolAdmin() external view returns (address) { return owner; }
    function getEmergencyAdmin() external view returns (address) { return owner; }
    function getOwners() external view returns (address[] memory owners) { owners = new address[](1); owners[0] = owner; }
    function getThreshold() external view returns (uint256) { return threshold; }
    function change(address next, uint256 minimum) external { owner = next; threshold = minimum; }
}
contract CookFixture {
    uint256 public debt = 50;
    uint256 public amount = 100;
    uint256 public exchangeRate = 1e18;
    uint256 public constant COLLATERIZATION_RATE = 75_000;
    function configure(uint256 debt_, uint256 rate_) external { debt = debt_; exchangeRate = rate_; }
    function userBorrowPart(address) external view returns (uint256) { return debt; }
    function userCollateralShare(address) external view returns (uint256) { return amount; }
    function totalBorrow() external pure returns (uint128, uint128) { return (100, 100); }
    function bentoBox() external view returns (address) { return address(this); }
    function collateral() external pure returns (address) { return address(0x1234); }
    function toAmount(address, uint256 shares, bool) external pure returns (uint256) { return shares; }
    function cook(uint8[] calldata, uint256[] calldata, bytes[] calldata) external {}
}
contract DonationFixture {
    mapping(address => uint256) public collateral;
    mapping(address => uint256) public liability;
    function configure(address account, uint256 c, uint256 l) external { collateral[account] = c; liability[account] = l; }
    function liquidity(address account) external view returns (uint256, uint256, uint256, bool) {
        return (collateral[account], liability[account], 0, false);
    }
    function donateToReserves(uint256 subAccount, uint256 amount) external {
        address account = address(uint160(msg.sender) ^ uint160(subAccount));
        collateral[account] -= amount;
    }
}
contract TreasuryFixture {
    uint256 public funds = 100;
    function balanceOf(address) external view returns (uint256) { return funds; }
    function pay(uint256 amount) external { funds -= amount; }
}
contract CaseAuthoritiesAndSolvencyTest is Test, CredibleTest {
    function radiant(RolesFixture t) internal { cl.assertion(address(t), type(RadiantAuthority).creationCode, RadiantAuthority.check.selector); }
    function testRadiantAllowsUnchanged() public { RolesFixture t = new RolesFixture(); radiant(t); t.change(address(0x1234), 1); }
    function testRadiantRejectsAuthorityChange() public {
        RolesFixture t = new RolesFixture(); radiant(t); vm.expectRevert(bytes("Authority changed")); t.change(address(0x5678), 1);
    }
    function safe(RolesFixture t) internal {
        address[] memory allowed = new address[](1); allowed[0] = address(0x1234);
        cl.assertion(address(t), abi.encodePacked(type(SafeConfiguration).creationCode, abi.encode(uint256(1), allowed)), SafeConfiguration.check.selector);
    }
    function testSafeAllowsApprovedConfiguration() public { RolesFixture t = new RolesFixture(); safe(t); t.change(address(0x1234), 1); }
    function testSafeRejectsUnknownOwner() public {
        RolesFixture t = new RolesFixture(); safe(t); vm.expectRevert(bytes("Unapproved owner")); t.change(address(0x5678), 1);
    }
    function testSafeRejectsThresholdReduction() public {
        RolesFixture t = new RolesFixture(); safe(t); vm.expectRevert(bytes("Unsafe threshold")); t.change(address(0x1234), 0);
    }
    function cook(CookFixture t) internal { cl.assertion(address(t), type(CookSolvency).creationCode, CookSolvency.check.selector); }
    function executeCook(CookFixture t) internal { t.cook(new uint8[](0), new uint256[](0), new bytes[](0)); }
    function testCookAllowsSolventAccount() public { CookFixture t = new CookFixture(); cook(t); executeCook(t); }
    function testCookAllowsNoDebt() public { CookFixture t = new CookFixture(); t.configure(0, 1e18); cook(t); executeCook(t); }
    function testCookRejectsUnsafeDebt() public {
        CookFixture t = new CookFixture(); t.configure(76, 1e18); cook(t); vm.expectRevert(bytes("Insolvent cook caller")); executeCook(t);
    }
    function testCookRespectsCollateralPerDebtRate() public {
        CookFixture t = new CookFixture(); t.configure(50, 2e18); cook(t); vm.expectRevert(bytes("Insolvent cook caller")); executeCook(t);
    }
    function donation(DonationFixture t) internal {
        cl.assertion(address(t), abi.encodePacked(type(EulerDonationHealth).creationCode, abi.encode(address(t))), EulerDonationHealth.check.selector);
    }
    function testDonationAllowsHealthyAccount() public {
        DonationFixture t = new DonationFixture(); t.configure(address(this), 100, 50); donation(t); t.donateToReserves(0, 40);
    }
    function testDonationRejectsUnsafeAccount() public {
        DonationFixture t = new DonationFixture(); t.configure(address(this), 100, 50); donation(t);
        vm.expectRevert(bytes("Donation leaves unsafe debt")); t.donateToReserves(0, 51);
    }
    function testDonationUsesRequestedSubaccount() public {
        DonationFixture t = new DonationFixture(); t.configure(address(uint160(address(this)) ^ uint160(1)), 100, 50); donation(t);
        vm.expectRevert(bytes("Donation leaves unsafe debt")); t.donateToReserves(1, 51);
    }
    function treasury(TreasuryFixture t) internal {
        cl.assertion(address(t), abi.encodePacked(type(RewardPayoutLimit).creationCode, abi.encode(address(t), uint256(10))), RewardPayoutLimit.check.selector);
    }
    function testRewardAllowsCap() public { TreasuryFixture t = new TreasuryFixture(); treasury(t); t.pay(10); }
    function testRewardRejectsAboveCap() public {
        TreasuryFixture t = new TreasuryFixture(); treasury(t); vm.expectRevert(bytes("Reward payout above cap")); t.pay(11);
    }
}
