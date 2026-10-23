import { useState } from 'react';

type RetainedPanel<TPanel> = {
	/** The open panel, or the last one while the drawer slides out; `null` before the first open. */
	panel: TPanel | null;
	isOpen: boolean;
	/** Changes on every open, so the content starts from a fresh state each time. */
	instanceKey: number;
};

type RetainedState<TPanel> = {
	panel: TPanel | null;
	instanceKey: number;
};

/** Keeps the last open panel while it is closing so its drawer can play the exit animation. */
export function useRetainedPanel<TPanel extends object>(
	openPanel: TPanel | null
): RetainedPanel<TPanel> {
	const [retained, setRetained] = useState<RetainedState<TPanel>>({
		panel: openPanel,
		instanceKey: 0,
	});
	if (openPanel !== null && openPanel !== retained.panel) {
		setRetained({ panel: openPanel, instanceKey: retained.instanceKey + 1 });
	}

	return {
		panel: openPanel ?? retained.panel,
		isOpen: openPanel !== null,
		instanceKey: retained.instanceKey,
	};
}
