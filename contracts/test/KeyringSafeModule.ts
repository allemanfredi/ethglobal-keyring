import { expect } from "chai"
import { AbiCoder } from "ethers"
import { ethers, upgrades, network } from "hardhat"
import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers"
import { AddressZero } from "@ethersproject/constants"

import { Operation } from "./utils/Operation"
import { Node } from "./utils/Node"
import {
  buildSafeTransaction,
  buildSignatureBytes,
  executeContractCallWithSigners,
  safeApproveHash,
} from "./utils/safe"

import { Token, SafeProxyFactory, Safe, KeyringSafeModule, KeyringGateway } from "../typechain-types/"

describe("KeyringSafeModule", () => {
  let gateway: KeyringGateway
  let module: KeyringSafeModule
  let node = new Node()
  let safe: Safe
  let factory: SafeProxyFactory
  let token: Token
  let abiCoder = new AbiCoder()
  let owner: SignerWithAddress
  let user1: SignerWithAddress
  let signer: SignerWithAddress

  describe("executeOperation", function () {
    this.beforeEach(async () => {
      const KeyringGateway = await ethers.getContractFactory("KeyringGateway")
      const signers = await ethers.getSigners()
      owner = signers[0]
      user1 = signers[1]
      signer = signers[2]
      const proxy1 = await upgrades.deployProxy(KeyringGateway, [owner.address])
      await proxy1.waitForDeployment()
      gateway = (await KeyringGateway.attach(await proxy1.getAddress())) as KeyringGateway

      // Safe setup
      const saltNumber = "1111"
      const Safe = await ethers.getContractFactory("Safe")
      const SafeProxyFactory = await ethers.getContractFactory("SafeProxyFactory")
      const singleton = (await Safe.deploy()) as Safe
      factory = (await SafeProxyFactory.deploy()) as SafeProxyFactory
      const template = await factory.createProxyWithNonce.staticCall(await singleton.getAddress(), "0x", saltNumber)
      await factory.createProxyWithNonce(await singleton.getAddress(), "0x", saltNumber).then((tx) => tx.wait())
      safe = singleton.attach(template) as Safe
      await safe.setup([owner.address], 1, AddressZero, "0x", AddressZero, AddressZero, 0, AddressZero)

      const KeyringSafeModule = await ethers.getContractFactory("KeyringSafeModule")
      const proxy2 = await upgrades.deployProxy(KeyringSafeModule, [
        owner.address,
        await gateway.getAddress(),
        await safe.getAddress(),
        signer.address,
      ])
      await proxy2.waitForDeployment()
      module = (await KeyringSafeModule.attach(await proxy2.getAddress())) as KeyringSafeModule

      await executeContractCallWithSigners(safe, safe, "enableModule", [await module.getAddress()], [owner])

      const Token = await ethers.getContractFactory("Token")
      token = await Token.deploy()
      await token.transfer(await safe.getAddress(), ethers.parseEther("100"))
    })

    it("should be able to transfer tokens using the KeyringModule", async () => {
      const amountToTransfer = ethers.parseEther("0.1")
      const tx = buildSafeTransaction({
        to: await token.getAddress(),
        safeTxGas: 1000000,
        nonce: await safe.nonce(),
        data: (await token.transfer.populateTransaction(user1.address, amountToTransfer)).data,
      })
      const signatureBytes = buildSignatureBytes([await safeApproveHash(owner, safe, tx, false)])
      const operationData = abiCoder.encode(
        ["address", "uint256", "bytes", "uint8", "uint256", "uint256", "uint256", "address", "address", "bytes"],
        [
          tx.to,
          tx.value,
          tx.data,
          tx.operation,
          tx.safeTxGas,
          tx.baseGas,
          tx.gasPrice,
          tx.gasToken,
          tx.refundReceiver,
          signatureBytes,
        ],
      )

      const operation = new Operation({
        protocol: "Evm",
        chainId: network.config.chainId!,
        targetAddress: await module.getAddress(),
        data: operationData,
      })

      const signedOperation = node.signOperation(operation)
      await gateway.enableSigner(node.wallet.address)
      await expect(gateway.executeOperation(operation.serialize(), signedOperation)).to.emit(
        gateway,
        "KeyringOperationExecuted",
      )
      const user1Balance = await token.balanceOf(user1.address)
      expect(user1Balance).to.be.eq(amountToTransfer)
    })
  })
})
