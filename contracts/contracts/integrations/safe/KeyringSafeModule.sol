// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { UUPSUpgradeable } from "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import { AccessControlEnumerableUpgradeable } from "@openzeppelin/contracts-upgradeable/access/extensions/AccessControlEnumerableUpgradeable.sol";
import { IModuleManager } from "safe-contracts/contracts/interfaces/IModuleManager.sol";
import { ISafe, Enum } from "safe-contracts/contracts/interfaces/ISafe.sol";
import { IKeyringTarget } from "../../interfaces/IKeyringTarget.sol";
import { Operation } from "../../interfaces/Operation.sol";
import { IKeyringSafeModule } from "./IKeyringSafeModule.sol";

contract KeyringSafeModule is IKeyringSafeModule, IKeyringTarget, UUPSUpgradeable, AccessControlEnumerableUpgradeable {
    bytes32 public constant UPDATE_GATEWAY_ROLE = keccak256("UPDATE_GATEWAY_ROLE");
    bytes32 public constant UPDATE_SAFE_ROLE = keccak256("UPDATE_SAFE_ROLE");
    bytes32 public constant ON_OPERATION_ROLE = keccak256("ON_OPERATION_ROLE");

    address public gateway;
    address public safe;

    function initialize(address owner, address gateway_, address safe_) public initializer {
        __AccessControlEnumerable_init();
        __UUPSUpgradeable_init();

        gateway = gateway_;
        safe = safe_;

        _grantRole(DEFAULT_ADMIN_ROLE, owner);
        _grantRole(UPDATE_GATEWAY_ROLE, owner);
        _grantRole(UPDATE_SAFE_ROLE, owner);
        _grantRole(ON_OPERATION_ROLE, gateway_);
    }

    function onOperation(Operation memory operation) external onlyRole(ON_OPERATION_ROLE) {
        (
            address to,
            uint256 value,
            bytes memory data,
            Enum.Operation safeOperation,
            uint256 safeTxGas,
            uint256 baseGas,
            uint256 gasPrice,
            address gasToken,
            address payable refundReceiver,
            bytes memory signatures
        ) = abi.decode(
                operation.data,
                (address, uint256, bytes, Enum.Operation, uint256, uint256, uint256, address, address, bytes)
            );

        address targetSafe = safe;
        IModuleManager(targetSafe).execTransactionFromModule(
            targetSafe,
            0,
            abi.encodeWithSelector(
                ISafe.execTransaction.selector,
                to,
                value,
                data,
                safeOperation,
                safeTxGas,
                baseGas,
                gasPrice,
                gasToken,
                refundReceiver,
                signatures
            ),
            Enum.Operation.Call
        );
    }

    function updateGateway(address newGateway) external onlyRole(UPDATE_GATEWAY_ROLE) {
        gateway = newGateway;
        emit GatewayUpdated(newGateway);
    }

    function updateSafe(address newSafe) external onlyRole(UPDATE_SAFE_ROLE) {
        safe = newSafe;
        emit SafeUpdated(newSafe);
    }

    function _authorizeUpgrade(address) internal override onlyRole(DEFAULT_ADMIN_ROLE) {}
}
