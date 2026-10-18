import { createContext } from 'react';

/** Called with the ids of the tunnels that were just deleted, so focus can move on. */
export const RemovalFocusContext = createContext<
	(tunnelIds: readonly string[]) => void
>(() => undefined);
