import { registerPlugin } from "../registry";
import GitPanel from "./GitPanel";
import manifest from "./manifest.json";

registerPlugin({
  ...manifest,
  component: GitPanel,
});
