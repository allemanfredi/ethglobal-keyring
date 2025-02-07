// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

interface IKeyringSafeModule {
    event GatewayUpdated(address gateway);
    event SafeUpdated(address safe);

    error InvalidSigner();

    function updateGateway(address newGateway) external;

    function updateSafe(address newSafe) external;
}
