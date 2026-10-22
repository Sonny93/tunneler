import { useEffect } from 'react';
import { useLingui } from '@lingui/react/macro';

import type { TunnelDto } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { buildStateAnnouncement } from '#/tunnels/tunnel_state_announcement';

export const ANNOUNCEMENT_THROTTLE_MS = 1000;

function findChangedTunnels(
	previousTunnels: readonly TunnelDto[],
	currentTunnels: readonly TunnelDto[]
): TunnelDto[] {
	const previousStates = new Map(
		previousTunnels.map((tunnel) => [tunnel.id, JSON.stringify(tunnel.state)])
	);
	return currentTunnels.filter((tunnel) => {
		const previousState = previousStates.get(tunnel.id);
		return (
			previousState !== undefined &&
			previousState !== JSON.stringify(tunnel.state)
		);
	});
}

/**
 * Announces tunnel state changes politely, collected over one throttle window.
 * Tunnels that were not known before (the initial snapshot) are never announced.
 */
export function useTunnelStateAnnouncements(): void {
	const { i18n } = useLingui();

	useEffect(() => {
		const pendingChanges = new Map<string, TunnelDto>();
		let timeoutId: ReturnType<typeof setTimeout> | undefined;

		const flush = () => {
			timeoutId = undefined;
			const message = buildStateAnnouncement(
				[...pendingChanges.values()],
				i18n
			);
			pendingChanges.clear();
			useAnnouncerStore.getState().announce(message, 'polite');
		};

		const unsubscribe = useDaemonStore.subscribe((state, previousState) => {
			if (state.tunnels === previousState.tunnels) return;
			for (const tunnel of findChangedTunnels(
				previousState.tunnels,
				state.tunnels
			)) {
				pendingChanges.set(tunnel.id, tunnel);
			}
			if (pendingChanges.size === 0 || timeoutId !== undefined) return;
			timeoutId = setTimeout(flush, ANNOUNCEMENT_THROTTLE_MS);
		});

		return () => {
			unsubscribe();
			clearTimeout(timeoutId);
		};
	}, [i18n]);
}
