import tsGenerator from "@cosmwasm/ts-codegen";
import { join } from "path";

const contractsPath = (path?: string) => join(__dirname, "../src/");
const outPath = join(__dirname, "./interfaces");

tsGenerator({
  contracts: [
    {
      name: "super_star",
      dir: contractsPath(),
    },
  ],
  outPath,
  options: {
    bundle: {
      enabled: false,
    },
  },
}).then(() => console.log("Generated typescript interfaces for contracts"));
