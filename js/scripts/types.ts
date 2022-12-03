import tsGenerator from "@cosmwasm/ts-codegen";
import { join } from "path";

const dir = join(__dirname, "../../schema");
const outPath = join(__dirname, "../types");

tsGenerator({
  contracts: [
    {
      name: "super_star",
      dir,
    },
  ],
  outPath,
  options: {
    bundle: {
      enabled: false,
    },
    client: {
      enabled: true,
    },
  },
}).then(() => console.log("Generated typescript interfaces for contracts"));
