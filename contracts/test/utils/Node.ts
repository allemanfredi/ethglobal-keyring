import { ethers } from "hardhat"
import { HDNodeWallet } from "ethers"
import { Operation } from "./Operation"

export class Node {
  wallet: HDNodeWallet

  constructor() {
    const wallet = ethers.Wallet.createRandom()
    this.wallet = wallet
  }

  signOperation(operation: Operation): string {
    const serialized = operation.serialize()
    const hash = ethers.sha256(serialized)
    return this.wallet.signingKey.sign(hash).serialized
  }
}
