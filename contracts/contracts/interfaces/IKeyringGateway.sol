// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { Operation, Protocol } from "./Operation.sol";

interface IKeyringGateway {
    event KeyringOperationExecuted(Operation operation);

    error OnOperationFailed();
    error OperationAlreadyExecuted();
    error InvalidChainId();
    error InvalidProtocol();

    function canBeExecuted(bytes calldata operation) external view returns (bool);

    function executeOperation(bytes calldata operation, bytes calldata signature) external;
}
