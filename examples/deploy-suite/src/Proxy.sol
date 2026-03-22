// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract SimpleProxy {
    address public implementation;
    address public admin;

    event Upgraded(address indexed newImpl);

    constructor(address _impl) {
        implementation = _impl;
        admin = msg.sender;
    }

    function upgrade(address _newImpl) external {
        require(msg.sender == admin, "not admin");
        implementation = _newImpl;
        emit Upgraded(_newImpl);
    }

    fallback() external payable {
        address impl = implementation;
        assembly {
            calldatacopy(0, 0, calldatasize())
            let result := delegatecall(gas(), impl, 0, calldatasize(), 0, 0)
            returndatacopy(0, 0, returndatasize())
            switch result
            case 0 { revert(0, returndatasize()) }
            default { return(0, returndatasize()) }
        }
    }

    receive() external payable {}
}
