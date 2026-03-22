// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract Auction {
    address public seller;
    uint256 public endTime;
    address public highestBidder;
    uint256 public highestBid;
    bool public ended;

    mapping(address => uint256) public pendingReturns;

    event Bid(address indexed bidder, uint256 amount);
    event AuctionEnded(address winner, uint256 amount);

    constructor(uint256 _duration) {
        seller = msg.sender;
        endTime = block.timestamp + _duration;
    }

    function bid() external payable {
        require(block.timestamp < endTime, "ended");
        require(msg.value > highestBid, "too low");
        if (highestBidder != address(0)) {
            pendingReturns[highestBidder] += highestBid;
        }
        highestBidder = msg.sender;
        highestBid = msg.value;
        emit Bid(msg.sender, msg.value);
    }

    function withdraw() external {
        uint256 amount = pendingReturns[msg.sender];
        require(amount > 0, "nothing");
        pendingReturns[msg.sender] = 0;
        (bool ok, ) = msg.sender.call{value: amount}("");
        require(ok, "failed");
    }

    function endAuction() external {
        require(block.timestamp >= endTime, "not ended");
        require(!ended, "already ended");
        ended = true;
        emit AuctionEnded(highestBidder, highestBid);
        (bool ok, ) = seller.call{value: highestBid}("");
        require(ok, "failed");
    }
}
