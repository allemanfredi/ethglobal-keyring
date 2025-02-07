// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

enum Protocol {
    Evm
}

struct Operation {
    Protocol protocol;
    uint64 chainId;
    address target;
    bytes data;
    bytes salt;
}
