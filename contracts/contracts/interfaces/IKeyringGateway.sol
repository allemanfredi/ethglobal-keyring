// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { Operation, Protocol } from "./Operation.sol";

interface IKeyringGateway {
    event OperationExecuted(Operation operation);
    event SignerEnabled(address signer);

    error OnOperationFailed();
    error OperationAlreadyExecuted();
    error InvalidChainId();
    error InvalidProtocol();
    error SignerNotEnabledOrInvalidSignature();

    function canBeExecuted(bytes calldata operation) external view returns (bool);

    function enableSigner(address signer) external;

    function executeOperation(bytes calldata operation, bytes calldata signature) external;
}
