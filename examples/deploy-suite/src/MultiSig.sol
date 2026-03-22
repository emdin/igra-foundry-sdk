// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract MultiSig {
    address[] public owners;
    uint256 public required;
    uint256 public txCount;

    struct Transaction {
        address to;
        uint256 value;
        bytes data;
        bool executed;
        uint256 confirmations;
    }

    mapping(uint256 => Transaction) public transactions;
    mapping(uint256 => mapping(address => bool)) public confirmed;

    event Submit(uint256 indexed txId);
    event Confirm(uint256 indexed txId, address indexed owner);
    event Execute(uint256 indexed txId);

    modifier onlyOwner() {
        bool isOwner = false;
        for (uint i = 0; i < owners.length; i++) {
            if (owners[i] == msg.sender) { isOwner = true; break; }
        }
        require(isOwner, "not owner");
        _;
    }

    constructor(address[] memory _owners, uint256 _required) {
        require(_owners.length > 0 && _required > 0 && _required <= _owners.length);
        owners = _owners;
        required = _required;
    }

    function submit(address _to, uint256 _value, bytes calldata _data) external onlyOwner returns (uint256) {
        uint256 txId = txCount++;
        transactions[txId] = Transaction(_to, _value, _data, false, 0);
        emit Submit(txId);
        return txId;
    }

    function confirm(uint256 _txId) external onlyOwner {
        require(!confirmed[_txId][msg.sender], "already confirmed");
        confirmed[_txId][msg.sender] = true;
        transactions[_txId].confirmations++;
        emit Confirm(_txId, msg.sender);
    }

    function execute(uint256 _txId) external onlyOwner {
        Transaction storage t = transactions[_txId];
        require(!t.executed && t.confirmations >= required, "cannot execute");
        t.executed = true;
        (bool ok, ) = t.to.call{value: t.value}(t.data);
        require(ok, "call failed");
        emit Execute(_txId);
    }

    receive() external payable {}
}
