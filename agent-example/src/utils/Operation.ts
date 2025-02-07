import * as borsh from "borsh"
import { ethers } from "ethers"
import crypto from "crypto"

const PROTOCOLS_ENUM = {
  evm: 0,
}

type Protocol = "evm"

interface OperationConfigs {
  protocol: Protocol
  chainId: number
  targetAddress: string
  data: string
}

export class Operation {
  protocol: Protocol
  chainId: number
  targetAddress: string
  data: string
  salt: string

  constructor({ protocol, chainId, targetAddress, data }: OperationConfigs) {
    this.protocol = protocol
    this.chainId = chainId
    this.targetAddress = targetAddress
    this.data = data
    this.salt = "0x" + crypto.randomBytes(10).toString("hex")
  }

  serialize(): Uint8Array {
    return borsh.serialize(
      {
        struct: {
          protocol: "u8", // ENUM
          chainId: "u64",
          targetAddress: { array: { type: "u8" } },
          data: { array: { type: "u8" } },
          salt: { array: { type: "u8" } },
        },
      },
      {
        protocol: PROTOCOLS_ENUM[this.protocol],
        chainId: this.chainId,
        targetAddress: ethers.getBytes(this.targetAddress),
        data: ethers.getBytes(this.data),
        salt: ethers.getBytes(this.salt),
      },
    )
  }

  encode() {
    return [PROTOCOLS_ENUM[this.protocol], this.chainId, this.targetAddress, this.data, this.salt]
  }
}
