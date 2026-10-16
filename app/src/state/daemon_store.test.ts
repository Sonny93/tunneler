import { describe, expect, it } from 'vitest';

import { useDaemonStore } from '#/state/daemon_store';
import { buildSnapshot, buildTunnel } from '#/testing/factories';

describe('useDaemonStore', () => {
	it('should start in the connecting status', () => {
		expect(useDaemonStore.getState().connection).toEqual({
			status: 'connecting',
		});
	});

	it('should load tunnels and settings from a snapshot', () => {
		const snapshot = buildSnapshot();

		useDaemonStore.getState().applySnapshot(snapshot);

		expect(useDaemonStore.getState()).toMatchObject({
			tunnels: snapshot.tunnels,
			settings: snapshot.settings,
		});
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
			data: { id: 'a', state: { kind: 'active' }, health: { kind: 'unknown' } },
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

	it('should clear the quit request', () => {
		useDaemonStore.getState().applyEvent({ event: 'quit.requested', data: {} });

		useDaemonStore.getState().clearQuitRequest();

		expect(useDaemonStore.getState().isQuitRequested).toBe(false);
	});
});
