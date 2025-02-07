// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { Operation, Protocol } from "../interfaces/Operation.sol";
import { Borsh } from "./Borsh.sol";

library OperationDecoder {
    using Borsh for Borsh.Data;
    using OperationDecoder for bytes;

    function decode(bytes memory operation) internal pure returns (Operation memory) {
        Borsh.Data memory encoded = Borsh.from(operation);
        uint8 protocol = encoded.decodeU8();
        uint64 chainId = encoded.decodeU64();
        address target = address(bytes20(encoded.decodeBytes()));
        bytes memory data = encoded.decodeBytes();
        bytes memory salt = encoded.decodeBytes();
        return Operation(Protocol(protocol), chainId, target, data, salt);
    }
}
