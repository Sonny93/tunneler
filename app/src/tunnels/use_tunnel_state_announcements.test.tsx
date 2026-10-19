import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useDaemonStore } from '#/state/daemon_store';
import { buildSnapshot, buildTunnel } from '#/testing/factories';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import {
	ANNOUNCEMENT_THROTTLE_MS,
	useTunnelStateAnnouncements,
} from '#/tunnels/use_tunnel_state_announcements';

const loadTunnels = (...ids: string[]) =>
	act(() =>
		useDaemonStore.getState().applySnapshot(
			buildSnapshot({
				tunnels: ids.map((id, index) =>
					buildTunnel({ id, listenPort: 3001 + index })
				),
			})
		)
	);

const activate = (id: string) =>
	act(() =>
		useDaemonStore.getState().applyEvent({
			event: 'tunnel.changed',
			data: { id, state: { kind: 'active' }, health: { kind: 'unknown' } },
		})
	);

const politeMessage = () => useAnnouncerStore.getState().polite.message;

describe('useTunnelStateAnnouncements', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('should not announce the initial snapshot', () => {
		renderHook(() => useTunnelStateAnnouncements());

		loadTunnels('a', 'b');
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS * 2));

		expect(politeMessage()).toBe('');
	});

	it('should announce a single change once the throttle window ends', () => {
		loadTunnels('a');
		renderHook(() => useTunnelStateAnnouncements());

		activate('a');
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS));

		expect(politeMessage()).toBe('3001 vers nova:3001 : Actif');
	});

	it('should wait for the end of the window before announcing', () => {
		loadTunnels('a');
		renderHook(() => useTunnelStateAnnouncements());

		activate('a');
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS - 1));

		expect(politeMessage()).toBe('');
	});

	it('should group several changes in one announcement', () => {
		loadTunnels('a', 'b');
		renderHook(() => useTunnelStateAnnouncements());

		activate('a');
		activate('b');
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS));

		expect(politeMessage()).toBe(
			"2 tunnels ont changé d'état. 3001 vers nova:3001 : Actif. 3002 vers nova:3002 : Actif"
		);
	});

	it('should ignore changes of health only', () => {
		loadTunnels('a');
		renderHook(() => useTunnelStateAnnouncements());

		act(() =>
			useDaemonStore.getState().applyEvent({
				event: 'tunnel.changed',
				data: {
					id: 'a',
					state: { kind: 'stopped' },
					health: { kind: 'reachable' },
				},
			})
		);
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS));

		expect(politeMessage()).toBe('');
	});

	it('should stop announcing after unmount', () => {
		loadTunnels('a');
		const { unmount } = renderHook(() => useTunnelStateAnnouncements());
		activate('a');

		unmount();
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS));

		expect(politeMessage()).toBe('');
	});
});
