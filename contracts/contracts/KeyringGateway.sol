// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { UUPSUpgradeable } from "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import { AccessControlEnumerableUpgradeable } from "@openzeppelin/contracts-upgradeable/access/extensions/AccessControlEnumerableUpgradeable.sol";
import { ECDSA } from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import { OperationDecoder } from "./libraries/OperationDecoder.sol";
import { IKeyringGateway, Operation, Protocol } from "./interfaces/IKeyringGateway.sol";
import { IKeyringTarget } from "./interfaces/IKeyringTarget.sol";

contract KeyringGateway is IKeyringGateway, UUPSUpgradeable, AccessControlEnumerableUpgradeable {
    using OperationDecoder for bytes;

    mapping(address => bool) private _enabledSigners;
    mapping(bytes32 => bool) private _executedOperations;

    function initialize(address owner) public initializer {
        __AccessControlEnumerable_init();
        __UUPSUpgradeable_init();
        _grantRole(DEFAULT_ADMIN_ROLE, owner);
    }

    function canBeExecuted(bytes calldata operation) external view returns (bool) {
        return _executedOperations[sha256(operation)];
    }

    function executeOperation(bytes calldata operation, bytes calldata signature) external {
        bytes32 operationHash = sha256(operation);
        require(!_executedOperations[operationHash], OperationAlreadyExecuted());
        _executedOperations[operationHash] = true;

        address signer = ECDSA.recover(operationHash, signature);

        Operation memory op = operation.decode();
        require(op.protocol == Protocol.Evm, InvalidProtocol());
        require(op.chainId == block.chainid, InvalidChainId());

        try IKeyringTarget(op.target).onOperation(signer, op) {} catch {
            revert OnOperationFailed();
        }

        emit KeyringOperationExecuted(op);
    }

    function _authorizeUpgrade(address) internal override onlyRole(DEFAULT_ADMIN_ROLE) {}
}
