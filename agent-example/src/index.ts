import "dotenv/config"
import axios from "axios"
import { ethers, hexlify } from "ethers"

import { Operation } from "./utils/Operation"
import { buildSafeTransaction, buildSignatureBytes, safeApproveHash } from "./utils/safe"
import safeAbi from "./utils/abi/Safe"
import erc20Abi from "./utils/abi/ERC20"

const stripHexPrefix = (_str: string) => (_str.startsWith("0x") ? _str.slice(2) : _str)

const SAFE_ADDRESS = "0x62D15808fA7a102Acc5a2A765336c85e01ae31b5"
const KEYRING_SAFE_MODULE_ADDRESS = "0x3EE695F059b52A42d6F78F24eeF48e0FCaA8361e"
const CHAIN_ID = 100
const USDC_ADDRESS = "0xDDAfbb505ad214D7b80b1f830fcCc89B60fb7A83"

const main = async () => {
  const provider = new ethers.JsonRpcProvider("https://rpc.gnosis.gateway.fm")
  const abiCoder = new ethers.AbiCoder()
  const safe = new ethers.Contract(SAFE_ADDRESS, safeAbi, provider)
  const usdc = new ethers.Contract(USDC_ADDRESS, erc20Abi, provider)
  const owner = new ethers.Wallet(process.env.INSTANCE_KEY_PRIVATE_KEY as string, provider)
  const instanceKeyWallet = new ethers.Wallet(process.env.INSTANCE_KEY_PRIVATE_KEY as string, provider) // NOTE: equal just for the MVP
  const amountToTransfer = "10000"

  // Generate the key
  console.log("generating the key ...")
  const {
    data: { result: keygenResult },
  } = await axios.post("http://localhost:3001", {
    id: 1,
    jsonrpc: "2.0",
    method: "keyring_generateKey",
    params: [
      {
        keyType: "secp256k1",
        publicKey: stripHexPrefix(instanceKeyWallet.signingKey.publicKey),
      },
    ],
  })

  console.log("shared public key: ", keygenResult.sharedPublicKey)
  console.log("signer: ", keygenResult.sharedEvmAddress)

  console.log("preparing the USDC transfer material ...")
  const tx = buildSafeTransaction({
    to: await usdc.getAddress(),
    safeTxGas: 0,
    nonce: await safe.nonce(),
    data: (await usdc.transfer.populateTransaction(process.env.RECEIVER_ADDRESS, amountToTransfer)).data,
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
    protocol: "evm",
    chainId: CHAIN_ID,
    targetAddress: KEYRING_SAFE_MODULE_ADDRESS,
    data: operationData,
  })

  const serializedOperation = operation.serialize()
  const operationSignature = instanceKeyWallet.signingKey.sign(ethers.sha256(serializedOperation)).serialized

  console.log("keyring network is signing ...")
  const { data: { result: signingResult} } = await axios.post("http://localhost:3001", {
    id: 1,
    jsonrpc: "2.0",
    method: "keyring_sign",
    params: [
      stripHexPrefix(keygenResult.sharedPublicKey),
      {
        protocol: operation.protocol,
        chainId: operation.chainId,
        targetAddress: stripHexPrefix(operation.targetAddress),
        data: stripHexPrefix(operation.data),
        salt: stripHexPrefix(operation.salt),
      },
      stripHexPrefix(operationSignature.slice(0, operationSignature.length - 2)),
    ],
  })

  console.log("serialized operation", hexlify(serializedOperation))
  console.log("signature", signingResult.signature)
}

main()
