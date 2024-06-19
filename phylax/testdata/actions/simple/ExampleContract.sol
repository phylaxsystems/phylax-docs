pragma solidity 0.8.12;

// import {Action} from "phylax-std/Action.sol";

// contract DeployExample is Action {
//     ExampleContract example;
//     function run() public {
//         vm.startBroadcast();
//         example = new ExampleContract();
//     }
// }

contract ExampleContract {
    bool internal $flag;

    event message(string);

    constructor() {
        emit message("constructed");
    }

    function killProtocol() public {
        require(
            msg.sender == 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266,
            "not auth"
        );
        $flag = true;
    }

    function flag() public returns (bool) {
        return $flag;
    }
}
