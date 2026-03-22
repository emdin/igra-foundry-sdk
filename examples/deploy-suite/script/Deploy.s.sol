// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "../src/Counter.sol";
import "../src/SimpleToken.sol";
import "../src/OZToken.sol";
import "../src/NFT.sol";
import "../src/Vault.sol";
import "../src/MultiSig.sol";
import "../src/Staking.sol";
import "../src/Auction.sol";
import "../src/OZGovernor.sol";
import "../src/Proxy.sol";
import "../src/DEX.sol";

contract DeployAll is Script {
    function run() external {
        vm.startBroadcast();

        // 1. Counter — minimal contract
        Counter counter = new Counter();
        console.log("Counter:", address(counter));

        // 2. SimpleToken — basic ERC-20
        SimpleToken simpleToken = new SimpleToken(1_000_000);
        console.log("SimpleToken:", address(simpleToken));

        // 3. OZToken — OZ ERC-20 + Burnable
        OZToken ozToken = new OZToken(1_000_000);
        console.log("OZToken:", address(ozToken));

        // 4. IgraNFT — OZ ERC-721
        IgraNFT nft = new IgraNFT();
        console.log("IgraNFT:", address(nft));

        // 5. Vault — native token vault
        Vault vault = new Vault();
        console.log("Vault:", address(vault));

        // 6. MultiSig — multi-signature wallet
        address[] memory owners = new address[](1);
        owners[0] = msg.sender;
        MultiSig multisig = new MultiSig(owners, 1);
        console.log("MultiSig:", address(multisig));

        // 7. Staking — stake OZToken
        Staking staking = new Staking(address(ozToken));
        console.log("Staking:", address(staking));

        // 8. Auction — 24h auction
        Auction auction = new Auction(86400);
        console.log("Auction:", address(auction));

        // 9. GovToken — OZ ERC-20 Votes + Permit
        GovToken govToken = new GovToken(1_000_000);
        console.log("GovToken:", address(govToken));

        // 10. SimpleProxy — delegatecall proxy (pointing to Counter)
        SimpleProxy proxy = new SimpleProxy(address(counter));
        console.log("SimpleProxy:", address(proxy));

        // 11. SimpleDEX — AMM with OZToken / GovToken pair
        SimpleDEX dex = new SimpleDEX(address(ozToken), address(govToken));
        console.log("SimpleDEX:", address(dex));

        vm.stopBroadcast();
    }
}
