import { createContext } from 'react';

import type { TunnelDto } from '#/daemon/protocol';

export type TunnelSelection = {
	selectedIds: ReadonlySet<string>;
	/** The selected tunnels, in the order of the list. */
	selectedTunnels: readonly TunnelDto[];
	toggle: (tunnelId: string) => void;
	selectRange: (tunnelId: string) => void;
};

const NO_SELECTION: TunnelSelection = {
	selectedIds: new Set(),
	selectedTunnels: [],
	toggle: () => undefined,
	selectRange: () => undefined,
};

/** What the rows of the list need to tick themselves and to open the selection menu. */
export const TunnelSelectionContext =
	createContext<TunnelSelection>(NO_SELECTION);
