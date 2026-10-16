import { useEffect, type RefObject } from 'react';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import { focusRowSwitch } from '#/tunnels/list/use_focus_after_removal';

/**
 * Moves focus to the switch of a just-created tunnel (WCAG 2.4.3) once its row
 * is rendered and the form drawer, which restores its own focus on unmount, is
 * gone. A tunnel hidden by the current filter never gets a row, so the request
 * is dropped and focus stays where the drawer left it.
 */
export function useFocusAfterCreation(
	rowIds: readonly string[],
	containerRef: RefObject<HTMLElement | null>
): void {
	const pendingTunnelId = useUiStore((state) => state.pendingFocusTunnelId);
	const isPanelOpen = useUiStore((state) => state.panel.kind !== 'none');
	const isPendingTunnelKnown = useDaemonStore((state) =>
		state.tunnels.some(({ id }) => id === pendingTunnelId)
	);
	const isPendingRowRendered =
		pendingTunnelId !== null && rowIds.includes(pendingTunnelId);

	useEffect(() => {
		if (pendingTunnelId === null || isPanelOpen) return;
		if (isPendingRowRendered && containerRef.current) {
			focusRowSwitch(containerRef.current, pendingTunnelId);
		}
		if (isPendingRowRendered || isPendingTunnelKnown) {
			useUiStore.getState().clearPendingFocus();
		}
	}, [
		pendingTunnelId,
		isPanelOpen,
		isPendingRowRendered,
		isPendingTunnelKnown,
		containerRef,
	]);
}
