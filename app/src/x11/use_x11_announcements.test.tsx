import { act, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { useDaemonStore } from '#/state/daemon_store';
import { I18nTestProvider } from '#/testing/i18n_test_provider';
import { useX11Announcements } from '#/x11/use_x11_announcements';

const politeMessage = () => useAnnouncerStore.getState().polite.message;

describe('useX11Announcements', () => {
	it('should announce an anchor becoming active politely', () => {
		useDaemonStore.setState({ connection: { status: 'connected' } });
		renderHook(() => useX11Announcements(), { wrapper: I18nTestProvider });

		act(() =>
			useDaemonStore.getState().applyEvent({
				event: 'x11.changed',
				data: {
					host: 'nova',
					state: { kind: 'active', display: 'localhost:10.0' },
				},
			})
		);

		expect(politeMessage()).toBe('X11 nova: active (localhost:10.0)');
	});

	it('should announce an anchor being stopped', () => {
		useDaemonStore.setState({
			connection: { status: 'connected' },
			x11Anchors: [{ host: 'nova', state: { kind: 'starting' } }],
		});
		renderHook(() => useX11Announcements(), { wrapper: I18nTestProvider });

		act(() =>
			useDaemonStore.getState().applyEvent({
				event: 'x11.changed',
				data: { host: 'nova', state: { kind: 'stopped' } },
			})
		);

		expect(politeMessage()).toBe('X11 nova: stopped');
	});

	it('should not announce anchors loaded while connecting', () => {
		renderHook(() => useX11Announcements(), { wrapper: I18nTestProvider });

		act(() =>
			useDaemonStore.setState({
				x11Anchors: [{ host: 'nova', state: { kind: 'starting' } }],
			})
		);

		expect(politeMessage()).toBe('');
	});
});
