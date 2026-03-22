// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract Staking {
    IERC20 public stakingToken;
    uint256 public rewardRate = 100; // per block per token staked (in wei)

    struct StakeInfo {
        uint256 amount;
        uint256 startBlock;
        uint256 rewardDebt;
    }

    mapping(address => StakeInfo) public stakes;
    uint256 public totalStaked;

    event Staked(address indexed user, uint256 amount);
    event Unstaked(address indexed user, uint256 amount, uint256 reward);

    constructor(address _token) {
        stakingToken = IERC20(_token);
    }

    function stake(uint256 amount) external {
        require(amount > 0, "zero stake");
        _claimReward();
        stakingToken.transferFrom(msg.sender, address(this), amount);
        stakes[msg.sender].amount += amount;
        stakes[msg.sender].startBlock = block.number;
        totalStaked += amount;
        emit Staked(msg.sender, amount);
    }

    function unstake(uint256 amount) external {
        require(stakes[msg.sender].amount >= amount, "insufficient");
        uint256 reward = _claimReward();
        stakes[msg.sender].amount -= amount;
        totalStaked -= amount;
        stakingToken.transfer(msg.sender, amount);
        emit Unstaked(msg.sender, amount, reward);
    }

    function pendingReward(address user) external view returns (uint256) {
        StakeInfo storage s = stakes[user];
        if (s.amount == 0) return 0;
        return (block.number - s.startBlock) * s.amount * rewardRate / 1e18;
    }

    function _claimReward() internal returns (uint256) {
        StakeInfo storage s = stakes[msg.sender];
        if (s.amount == 0) return 0;
        uint256 reward = (block.number - s.startBlock) * s.amount * rewardRate / 1e18;
        s.startBlock = block.number;
        s.rewardDebt += reward;
        if (reward > 0) {
            stakingToken.transfer(msg.sender, reward);
        }
        return reward;
    }
}
