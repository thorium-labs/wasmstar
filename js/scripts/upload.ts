import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
import path from "path";
import fs from "fs";

import chains from "../config/chains";

(async () => {
  const mnemonic = process.env.MNEMONIC;
  const config = chains[process.env.CHAIN as keyof typeof chains];
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(
    mnemonic as string,
    { prefix: config.bech32Prefix }
  );
  const client = await SigningCosmWasmClient.connectWithSigner(
    config.rpcUrl,
    wallet,
    {
      prefix: config.bech32Prefix,
      gasPrice: GasPrice.fromString(
        config.defaultGasPrice + config.defaultFeeToken
      ),
    }
  );

  const wasmByte = fs.readFileSync(
    path.join(__dirname, "../../artifacts/super_star.wasm")
  );
  const [{ address }] = await wallet.getAccounts();
  const { codeId } = await client.upload(address, wasmByte, "auto");
  console.log("Code ID:", codeId);
})();
