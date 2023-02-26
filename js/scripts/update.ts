import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { DirectSecp256k1HdWallet } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
import chains from "../config/chains";
import { ExecuteMsg } from "../types/SuperStar.types";

const update = async () => {
    const mnemonic = process.env.MNEMONIC;
    const config = chains[process.env.CHAIN as keyof typeof chains];

    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic as string, { prefix: config.bech32Prefix });
    const client = await SigningCosmWasmClient.connectWithSigner(config.rpcUrl, wallet, {
        prefix: config.bech32Prefix,
        gasPrice: GasPrice.fromString(config.defaultGasPrice + config.defaultFeeToken),
    });
    const [{ address }] = await wallet.getAccounts();
    const msg: ExecuteMsg = { update_config: { new_config: { interval: { time: 60 * 30 } } } }
    const tx = await client.execute(address, process.env.CONTRACT_ADDR as string, msg, "auto")
    console.log(tx);
}

update();