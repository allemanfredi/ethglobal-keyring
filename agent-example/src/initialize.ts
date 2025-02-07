import "dotenv/config"
import axios from "axios"
import { ethers } from "ethers"

const stripHexPrefix = (_str: string) => (_str.startsWith("0x") ? _str.slice(2) : _str)

const main = async () => {
  const provider = new ethers.JsonRpcProvider("https://rpc.gnosis.gateway.fm")
  const instanceKeyWallet = new ethers.Wallet(process.env.INSTANCE_KEY_PRIVATE_KEY as string, provider) // NOTE: equal just for the MVP

  // Generate the key
  console.log("generating a new Keyring key ...")
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

  console.log(" shared public key:", keygenResult.sharedPublicKey)
  console.log(" signer:", keygenResult.sharedEvmAddress)
}

main()
