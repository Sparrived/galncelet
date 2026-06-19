import type { FC } from "react";

/** Definition of a widget panel. */
export interface PanelDef {
  /** Unique identifier (used as settings key) */
  id: string;
  /** Display title shown in the panel header */
  title: string;
  /** Height consumed when window is collapsed (logical px). 0 = hide entirely. */
  collapsedHeight: number;
  /** The panel React component */
  component: FC;
}
