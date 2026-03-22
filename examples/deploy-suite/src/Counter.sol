// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract Counter {
    uint256 public count;
    event CountChanged(uint256 newCount);

    function increment() external {
        count += 1;
        emit CountChanged(count);
    }

    function decrement() external {
        require(count > 0, "underflow");
        count -= 1;
        emit CountChanged(count);
    }

    function reset() external {
        count = 0;
        emit CountChanged(0);
    }
}
