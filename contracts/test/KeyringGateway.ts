import { expect } from "chai"
import { ethers, upgrades, network } from "hardhat"
import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers"

import { Operation } from "./utils/Operation"
import { Node } from "./utils/Node"

import { MockTarget, KeyringGateway } from "../typechain-types"

describe("KeyringGateway", () => {
  let gateway: KeyringGateway
  let target: MockTarget
  let owner: SignerWithAddress
  let node: Node

  describe("deployment", function () {
    this.beforeEach(async () => {
      const KeyringGateway = await ethers.getContractFactory("KeyringGateway")
      const [owner] = await ethers.getSigners()
      const proxy = await upgrades.deployProxy(KeyringGateway, [owner.address])
      await proxy.waitForDeployment()
      gateway = (await KeyringGateway.attach(await proxy.getAddress())) as KeyringGateway
    })
  })

  describe("executeOperation", function () {
    this.beforeEach(async () => {
      const KeyringGateway = await ethers.getContractFactory("KeyringGateway")
      const MockTarget = await ethers.getContractFactory("MockTarget")
      const signers = await ethers.getSigners()
      owner = signers[0]
      const proxy = await upgrades.deployProxy(KeyringGateway, [owner.address])
      await proxy.waitForDeployment()
      gateway = (await KeyringGateway.attach(await proxy.getAddress())) as KeyringGateway
      target = (await MockTarget.deploy()) as MockTarget
      node = new Node()
    })

    it("should be able to execute a valid operation", async () => {
      const operation = new Operation({
        protocol: "Evm",
        chainId: network.config.chainId!,
        targetAddress: await target.getAddress(),
        data: "0x0001",
      })
      const signedOperation = node.signOperation(operation)
      await expect(gateway.enableSigner(node.wallet.address))
        .to.emit(gateway, "SignerEnabled")
        .withArgs(node.wallet.address)
      await expect(gateway.executeOperation(operation.serialize(), signedOperation))
        .to.emit(gateway, "OperationExecuted")
        .withArgs(operation.encode())
        .and.to.emit(target, "OperationReceived")
    })

    it("should not be able to execute a valid operation twice using the same signature", async () => {
      const operation = new Operation({
        protocol: "Evm",
        chainId: network.config.chainId!,
        targetAddress: await target.getAddress(),
        data: "0x0001",
      })
      const signedOperation = node.signOperation(operation)
      await gateway.enableSigner(node.wallet.address)
      await gateway.executeOperation(operation.serialize(), signedOperation)
      await expect(gateway.executeOperation(operation.serialize(), signedOperation)).to.be.revertedWithCustomError(
        gateway,
        "OperationAlreadyExecuted",
      )
    })

    it("should not be able to execute a valid operation using a wrong signature", async () => {
      const operation = new Operation({
        protocol: "Evm",
        chainId: network.config.chainId!,
        targetAddress: await target.getAddress(),
        data: "0x0001",
      })
      const fakeOperation = new Operation({
        protocol: "Evm",
        chainId: network.config.chainId!,
        targetAddress: await target.getAddress(),
        data: "0x0002",
      })
      const signedOperation = node.signOperation(operation)
      await gateway.enableSigner(node.wallet.address)
      await expect(gateway.executeOperation(fakeOperation.serialize(), signedOperation)).to.be.revertedWithCustomError(
        gateway,
        "SignerNotEnabledOrInvalidSignature",
      )
    })
  })
})
