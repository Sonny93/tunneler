import { createContext } from 'react';

/** Called with the id of a tunnel that was just deleted, so focus can move on. */
export const RemovalFocusContext = createContext<(tunnelId: string) => void>(
	() => undefined
);
