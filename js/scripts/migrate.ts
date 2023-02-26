import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";

import chains from "../config/chains";

(async () => {
  const mnemonic = process.env.MNEMONIC;
  const config = chains[process.env.CHAIN as keyof typeof chains];
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic as string, { prefix: config.bech32Prefix });
  const client = await SigningCosmWasmClient.connectWithSigner(config.rpcUrl, wallet, {
    prefix: config.bech32Prefix,
    gasPrice: GasPrice.fromString(config.defaultGasPrice + config.defaultFeeToken),
  });

  const contractAddr = process.env.CONTRACT_ADDR as string;
  const codeId = process.env.CODE_ID as string;

  const [{ address }] = await wallet.getAccounts();
  console.log("Migrating contract", contractAddr, "to code", codeId);
  const result = await client.migrate(address, contractAddr, parseInt(codeId), {}, "auto");
  console.log(result);
})();
