import { registerPlugin } from "../registry";
import AmkrPanel from "./AmkrPanel";
import manifest from "./manifest.json";

registerPlugin({
  ...manifest,
  component: AmkrPanel,
});
