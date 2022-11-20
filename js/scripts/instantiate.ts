import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";

import chains from "../config/chains";
import NoisAddresses from "../config/nois";
import { InstantiateMsg } from "../types/SuperStar.types";

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

  const codeId = process.env.CODE_ID as string;
  const nois_proxy =
    NoisAddresses[process.env.CHAIN as keyof typeof NoisAddresses];
  const msg: InstantiateMsg = {
    lottery_interval: {
      // In seconds
      time: 60 * 60,
    },
    max_tickets_per_user: 100,
    nois_proxy,
    percentage_per_match: [3, 6, 8, 15, 25, 40],
    ticket_price: {
      denom: config.defaultFeeToken,
      amount: String(1e6),
    },
    treasury_fee: {
      denom: config.defaultFeeToken,
      amount: String(1e6),
    },
  };

  const [{ address }] = await wallet.getAccounts();
  const { contractAddress } = await client.instantiate(
    address,
    +codeId,
    msg,
    "super_start.v1",
    "auto"
  );
  console.log("Contract Address:", contractAddress);
})();
