// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract SimpleDEX is ERC20 {
    IERC20 public tokenA;
    IERC20 public tokenB;

    event LiquidityAdded(address indexed provider, uint256 amountA, uint256 amountB, uint256 liquidity);
    event Swap(address indexed user, address tokenIn, uint256 amountIn, uint256 amountOut);

    constructor(address _tokenA, address _tokenB) ERC20("IgraDEX-LP", "IDLP") {
        tokenA = IERC20(_tokenA);
        tokenB = IERC20(_tokenB);
    }

    function addLiquidity(uint256 amountA, uint256 amountB) external returns (uint256 liquidity) {
        tokenA.transferFrom(msg.sender, address(this), amountA);
        tokenB.transferFrom(msg.sender, address(this), amountB);

        uint256 supply = totalSupply();
        if (supply == 0) {
            liquidity = sqrt(amountA * amountB);
        } else {
            uint256 resA = tokenA.balanceOf(address(this)) - amountA;
            uint256 resB = tokenB.balanceOf(address(this)) - amountB;
            liquidity = min(amountA * supply / resA, amountB * supply / resB);
        }
        require(liquidity > 0, "zero liquidity");
        _mint(msg.sender, liquidity);
        emit LiquidityAdded(msg.sender, amountA, amountB, liquidity);
    }

    function swapAForB(uint256 amountIn) external returns (uint256 amountOut) {
        uint256 resA = tokenA.balanceOf(address(this));
        uint256 resB = tokenB.balanceOf(address(this));
        amountOut = (amountIn * 997 * resB) / (resA * 1000 + amountIn * 997);
        require(amountOut > 0, "insufficient output");
        tokenA.transferFrom(msg.sender, address(this), amountIn);
        tokenB.transfer(msg.sender, amountOut);
        emit Swap(msg.sender, address(tokenA), amountIn, amountOut);
    }

    function swapBForA(uint256 amountIn) external returns (uint256 amountOut) {
        uint256 resA = tokenA.balanceOf(address(this));
        uint256 resB = tokenB.balanceOf(address(this));
        amountOut = (amountIn * 997 * resA) / (resB * 1000 + amountIn * 997);
        require(amountOut > 0, "insufficient output");
        tokenB.transferFrom(msg.sender, address(this), amountIn);
        tokenA.transfer(msg.sender, amountOut);
        emit Swap(msg.sender, address(tokenB), amountIn, amountOut);
    }

    function sqrt(uint256 x) internal pure returns (uint256 y) {
        if (x == 0) return 0;
        y = x;
        uint256 z = (x + 1) / 2;
        while (z < y) { y = z; z = (x / z + z) / 2; }
    }

    function min(uint256 a, uint256 b) internal pure returns (uint256) {
        return a < b ? a : b;
    }
}
