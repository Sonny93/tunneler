import { describe, expect, it } from 'vitest';

import { useDaemonStore } from '#/state/daemon_store';
import {
	buildExternalProcess,
	buildSnapshot,
	buildTunnel,
	buildX11Anchor,
	buildX11Launcher,
} from '#/testing/factories';

describe('useDaemonStore', () => {
	it('should start in the connecting status', () => {
		expect(useDaemonStore.getState().connection).toEqual({
			status: 'connecting',
		});
	});

	it('should load tunnels and settings from a snapshot', () => {
		const snapshot = buildSnapshot({ externals: [buildExternalProcess()] });

		useDaemonStore.getState().applySnapshot(snapshot);

		expect(useDaemonStore.getState()).toMatchObject({
			tunnels: snapshot.tunnels,
			externals: snapshot.externals,
			settings: snapshot.settings,
		});
	});

	it('should load the X11 anchors and launchers from a snapshot', () => {
		const snapshot = buildSnapshot({
			x11Anchors: [buildX11Anchor()],
			x11Launchers: [buildX11Launcher()],
		});

		useDaemonStore.getState().applySnapshot(snapshot);

		expect(useDaemonStore.getState()).toMatchObject({
			x11Anchors: snapshot.x11Anchors,
			x11Launchers: snapshot.x11Launchers,
		});
	});

	it('should load the tray status from a snapshot', () => {
		useDaemonStore
			.getState()
			.applySnapshot(buildSnapshot({ tray: { kind: 'missingHost' } }));

		expect(useDaemonStore.getState().tray).toEqual({ kind: 'missingHost' });
	});

	it('should keep a quit request received before the first snapshot', () => {
		useDaemonStore.getState().applyEvent({ event: 'quit.requested', data: {} });

		useDaemonStore.getState().applySnapshot(buildSnapshot());

		expect(useDaemonStore.getState().isQuitRequested).toBe(true);
	});

	it('should clear configuration errors when a snapshot is applied', () => {
		useDaemonStore.getState().applyEvent({
			event: 'config.invalid',
			data: { messages: ['oops'] },
		});

		useDaemonStore.getState().applySnapshot(buildSnapshot());

		expect(useDaemonStore.getState().configErrors).toEqual([]);
	});

	it('should apply events through the reducer', () => {
		useDaemonStore
			.getState()
			.applySnapshot(buildSnapshot({ tunnels: [buildTunnel({ id: 'a' })] }));

		useDaemonStore.getState().applyEvent({
			event: 'tunnel.changed',
			data: {
				id: 'a',
				state: { kind: 'active' },
				health: { kind: 'unknown' },
				isAwake: false,
			},
		});

		expect(useDaemonStore.getState().tunnels[0]?.state).toEqual({
			kind: 'active',
		});
	});

	it('should dismiss a prompt by its promptId', () => {
		useDaemonStore.getState().applyEvent({
			event: 'credential.request',
			data: { promptId: 'p1', kind: 'passphrase', alias: 'a', keyPath: '/k' },
		});

		useDaemonStore.getState().dismissPrompt('p1');

		expect(useDaemonStore.getState().prompts).toEqual([]);
	});

	it('should dismiss an alert by its alertId', () => {
		useDaemonStore.getState().applyEvent({
			event: 'hostkey.request',
			data: {
				kind: 'alert',
				alias: 'a',
				reason: { kind: 'unknownRejectedByPolicy' },
				presentedFingerprint: 'SHA256:x',
			},
		});
		const [alert] = useDaemonStore.getState().prompts;

		useDaemonStore
			.getState()
			.dismissPrompt(alert && 'alertId' in alert ? alert.alertId : '');

		expect(useDaemonStore.getState().prompts).toEqual([]);
	});

	it('should clear the standby request', () => {
		useDaemonStore
			.getState()
			.applyEvent({ event: 'standby.requested', data: {} });

		useDaemonStore.getState().clearStandbyRequest();

		expect(useDaemonStore.getState().isStandbyRequested).toBe(false);
	});

	it('should clear the quit request', () => {
		useDaemonStore.getState().applyEvent({ event: 'quit.requested', data: {} });

		useDaemonStore.getState().clearQuitRequest();

		expect(useDaemonStore.getState().isQuitRequested).toBe(false);
	});
});
