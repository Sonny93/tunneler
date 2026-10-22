import { useEffect } from 'react';
import { useLingui } from '@lingui/react/macro';

import { findX11State } from '#/x11/find_x11_state';
import type { X11AnchorDto } from '#/daemon/protocol';
import { useDaemonStore } from '#/state/daemon_store';
import { buildX11Announcement } from '#/x11/x11_announcement';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';

function findChangedHosts(
	previousAnchors: readonly X11AnchorDto[],
	currentAnchors: readonly X11AnchorDto[]
): string[] {
	const hosts = new Set(
		[...previousAnchors, ...currentAnchors].map((anchor) => anchor.host)
	);
	return [...hosts].filter(
		(host) =>
			JSON.stringify(findX11State(previousAnchors, host)) !==
			JSON.stringify(findX11State(currentAnchors, host))
	);
}

/** Announces X11 anchor changes politely; the snapshot loaded while connecting is never announced. */
export function useX11Announcements(): void {
	const { i18n } = useLingui();

	useEffect(
		() =>
			useDaemonStore.subscribe((state, previousState) => {
				if (state.x11Anchors === previousState.x11Anchors) return;
				if (state.connection.status !== 'connected') return;
				for (const host of findChangedHosts(
					previousState.x11Anchors,
					state.x11Anchors
				)) {
					useAnnouncerStore
						.getState()
						.announce(
							buildX11Announcement(
								host,
								findX11State(state.x11Anchors, host),
								i18n
							),
							'polite'
						);
				}
			}),
		[i18n]
	);
}
