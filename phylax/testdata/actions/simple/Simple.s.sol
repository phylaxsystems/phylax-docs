pragma solidity 0.8.12;
import {Action} from "../lib/phylax-std/src/Action.sol";
import {ExampleContract} from "./ExampleContract.sol";

contract SimpleAction is Action {
    uint256 $anvil;
    address $exampleContractAddress =
        0x000000000000000000000000000000000000bEEF;
    ExampleContract $target;

    function run() public {
        $anvil = enableChain("anvil");
        vm.selectFork($activeChains[$anvil]);
        $target = ExampleContract($exampleContractAddress);
        require(!$target.flag());
        vm.startBroadcast();
        $target.killProtocol();
        vm.stopBroadcast();
        require($target.flag());
    }
}

contract SimpleActionRevert is Action {
    uint256 $anvil;
    address $exampleContractAddress =
        0x000000000000000000000000000000000000bEEF;
    ExampleContract $target;

    function run() public {
        $anvil = enableChain("anvil");
        vm.selectFork($activeChains[$anvil]);
        require(!$target.flag());
        $target = ExampleContract($exampleContractAddress);
        vm.startBroadcast();
        $target.killProtocol();
        vm.stopBroadcast();
        require($target.flag());
    }
}
