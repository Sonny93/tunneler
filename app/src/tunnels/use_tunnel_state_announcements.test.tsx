import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useDaemonStore } from '#/state/daemon_store';
import { I18nTestProvider } from '#/testing/i18n_test_provider';
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
			data: {
				id,
				state: { kind: 'active' },
				health: { kind: 'unknown' },
				isAwake: false,
			},
		})
	);

const renderTunnelStateAnnouncements = () =>
	renderHook(() => useTunnelStateAnnouncements(), {
		wrapper: I18nTestProvider,
	});

const politeMessage = () => useAnnouncerStore.getState().polite.message;

describe('useTunnelStateAnnouncements', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('should not announce the initial snapshot', () => {
		renderTunnelStateAnnouncements();

		loadTunnels('a', 'b');
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS * 2));

		expect(politeMessage()).toBe('');
	});

	it('should announce a single change once the throttle window ends', () => {
		loadTunnels('a');
		renderTunnelStateAnnouncements();

		activate('a');
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS));

		expect(politeMessage()).toBe('3001 to nova:3001: Active');
	});

	it('should wait for the end of the window before announcing', () => {
		loadTunnels('a');
		renderTunnelStateAnnouncements();

		activate('a');
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS - 1));

		expect(politeMessage()).toBe('');
	});

	it('should group several changes in one announcement', () => {
		loadTunnels('a', 'b');
		renderTunnelStateAnnouncements();

		activate('a');
		activate('b');
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS));

		expect(politeMessage()).toBe(
			'2 tunnels changed state. 3001 to nova:3001: Active. 3002 to nova:3002: Active'
		);
	});

	it('should ignore changes of health only', () => {
		loadTunnels('a');
		renderTunnelStateAnnouncements();

		act(() =>
			useDaemonStore.getState().applyEvent({
				event: 'tunnel.changed',
				data: {
					id: 'a',
					state: { kind: 'stopped' },
					health: { kind: 'reachable' },
					isAwake: false,
				},
			})
		);
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS));

		expect(politeMessage()).toBe('');
	});

	it('should stop announcing after unmount', () => {
		loadTunnels('a');
		const { unmount } = renderTunnelStateAnnouncements();
		activate('a');

		unmount();
		void act(() => vi.advanceTimersByTime(ANNOUNCEMENT_THROTTLE_MS));

		expect(politeMessage()).toBe('');
	});
});
