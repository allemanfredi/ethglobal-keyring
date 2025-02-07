// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { IKeyringTarget, Operation } from "../interfaces/IKeyringTarget.sol";

contract MockTarget is IKeyringTarget {
    event OperationReceived(address signer, Operation operation);

    function onOperation(address signer, Operation calldata operation) external {
        emit OperationReceived(signer, operation);
    }
}
