// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { IKeyringTarget, Operation } from "../interfaces/IKeyringTarget.sol";

contract MockTarget is IKeyringTarget {
    event OperationReceived(Operation operation);

    function onOperation(Operation calldata operation) external {
        emit OperationReceived(operation);
    }
}
