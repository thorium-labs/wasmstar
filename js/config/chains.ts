const juno_testnet = {
  chainId: "uni-5",
  chainName: "junotestnet",
  prettyName: "Juno Testnet",
  bech32Prefix: "juno",
  rpcUrl: "https://rpc.uni.juno.deuslabs.fi:443",
  restUrl: "https://lcd.uni.juno.deuslabs.fi",
  bip44: {
    coinType: 118,
  },
  defaultFeeToken: "ujunox",
  feeTokens: [
    {
      denom: "ujunox",
      coinDecimals: 6,
    },
  ],
  stakingToken: "ujunox",
  defaultGasPrice: 0.04,
  gasPriceStep: {
    low: 0.03,
    average: 0.04,
    high: 0.05,
  },
};

const osmosis_testnet = {
  chainId: "osmo-test-4",
  chainName: "osmosistestnet",
  prettyName: "Osmosis Testnet",
  bech32Prefix: "osmo",
  rpcUrl: "https://testnet-rpc.osmosis.zone/",
  restUrl: "https://testnet-rest.osmosis.zone/",
  bip44: {
    coinType: 118,
  },
  defaultFeeToken: "uosmo",
  feeTokens: [
    {
      denom: "uosmo",
      coinDecimals: 6,
    },
  ],
  stakingToken: "uosmo",
  defaultGasPrice: 0.025,
  gasPriceStep: {
    low: 0,
    average: 0.025,
    high: 0.04,
  },
};

const chains = { osmosis_testnet, juno_testnet };

export type Chains = typeof chains;
export type Chain = typeof chains[keyof typeof chains];
export default chains;
