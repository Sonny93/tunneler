import { describe, expect, it } from 'vitest';

import { reduceDaemonEvent } from '#/state/reduce_daemon_event';
import { buildMetrics, buildTunnel } from '#/testing/factories';
import { INITIAL_DAEMON_STATE, type DaemonState } from '#/state/daemon_state';

const stateWithTunnels = (...tunnels: DaemonState['tunnels'][number][]) => ({
	...INITIAL_DAEMON_STATE,
	tunnels,
});

const passwordRequestEvent = (promptId: string) =>
	({
		event: 'credential.request',
		data: {
			promptId,
			kind: 'password',
			alias: 'nova',
			user: 'root',
			host: 'nova.example',
		},
	}) as const;

describe('reduceDaemonEvent', () => {
	it('should patch state and health of the tunnel on tunnel.changed', () => {
		const state = stateWithTunnels(
			buildTunnel({ id: 'a' }),
			buildTunnel({ id: 'b' })
		);

		const next = reduceDaemonEvent(state, {
			event: 'tunnel.changed',
			data: {
				id: 'b',
				state: { kind: 'active' },
				health: { kind: 'reachable' },
			},
		});

		expect(next.tunnels.map((tunnel) => tunnel.state.kind)).toEqual([
			'stopped',
			'active',
		]);
		expect(next.tunnels[1]?.health).toEqual({ kind: 'reachable' });
	});

	it('should ignore tunnel.changed for an unknown id', () => {
		const state = stateWithTunnels(buildTunnel({ id: 'a' }));

		const next = reduceDaemonEvent(state, {
			event: 'tunnel.changed',
			data: {
				id: 'zzz',
				state: { kind: 'active' },
				health: { kind: 'unknown' },
			},
		});

		expect(next).toBe(state);
	});

	it('should patch metrics on tunnel.metrics', () => {
		const state = stateWithTunnels(buildTunnel({ id: 'a' }));
		const metrics = buildMetrics({ bytesIn: 42 });

		const next = reduceDaemonEvent(state, {
			event: 'tunnel.metrics',
			data: { id: 'a', metrics },
		});

		expect(next.tunnels[0]?.metrics).toEqual(metrics);
	});

	it('should ignore tunnel.metrics for an unknown id', () => {
		const state = stateWithTunnels(buildTunnel({ id: 'a' }));

		const next = reduceDaemonEvent(state, {
			event: 'tunnel.metrics',
			data: { id: 'zzz', metrics: buildMetrics() },
		});

		expect(next).toBe(state);
	});

	it('should ignore tunnel.log', () => {
		const next = reduceDaemonEvent(INITIAL_DAEMON_STATE, {
			event: 'tunnel.log',
			data: { id: 'a', entry: { at: 1, level: 'info', message: 'hi' } },
		});

		expect(next).toBe(INITIAL_DAEMON_STATE);
	});

	it('should ignore config.reloaded', () => {
		const next = reduceDaemonEvent(INITIAL_DAEMON_STATE, {
			event: 'config.reloaded',
			data: {},
		});

		expect(next).toBe(INITIAL_DAEMON_STATE);
	});

	it('should store the messages on config.invalid', () => {
		const next = reduceDaemonEvent(INITIAL_DAEMON_STATE, {
			event: 'config.invalid',
			data: { messages: ['ligne 3 invalide'] },
		});

		expect(next.configErrors).toEqual(['ligne 3 invalide']);
	});

	it('should append a credential prompt on credential.request', () => {
		const next = reduceDaemonEvent(
			INITIAL_DAEMON_STATE,
			passwordRequestEvent('p1')
		);

		expect(next.prompts).toEqual([
			{
				kind: 'credential',
				promptId: 'p1',
				request: {
					kind: 'password',
					alias: 'nova',
					user: 'root',
					host: 'nova.example',
				},
			},
		]);
	});

	it('should not duplicate a prompt with the same promptId', () => {
		const once = reduceDaemonEvent(
			INITIAL_DAEMON_STATE,
			passwordRequestEvent('p1')
		);

		const twice = reduceDaemonEvent(once, passwordRequestEvent('p1'));

		expect(twice.prompts).toHaveLength(1);
	});

	it('should keep prompts in arrival order', () => {
		const first = reduceDaemonEvent(
			INITIAL_DAEMON_STATE,
			passwordRequestEvent('p1')
		);

		const second = reduceDaemonEvent(first, passwordRequestEvent('p2'));

		expect(
			second.prompts.map((prompt) => 'promptId' in prompt && prompt.promptId)
		).toEqual(['p1', 'p2']);
	});

	it('should append a host key prompt on hostkey.request prompt', () => {
		const next = reduceDaemonEvent(INITIAL_DAEMON_STATE, {
			event: 'hostkey.request',
			data: {
				kind: 'prompt',
				promptId: 'p1',
				alias: 'nova',
				lookupName: 'nova.example',
				algorithm: 'ssh-ed25519',
				fingerprintSha256: 'SHA256:abc',
			},
		});

		expect(next.prompts).toEqual([
			{
				kind: 'hostKey',
				promptId: 'p1',
				alias: 'nova',
				lookupName: 'nova.example',
				algorithm: 'ssh-ed25519',
				fingerprintSha256: 'SHA256:abc',
			},
		]);
	});

	it('should append a host key alert with a generated alertId', () => {
		const next = reduceDaemonEvent(INITIAL_DAEMON_STATE, {
			event: 'hostkey.request',
			data: {
				kind: 'alert',
				alias: 'nova',
				reason: { kind: 'unknownRejectedByPolicy' },
				presentedFingerprint: 'SHA256:abc',
			},
		});

		expect(next.prompts[0]).toMatchObject({
			kind: 'hostKeyAlert',
			alertId: expect.any(String),
			alias: 'nova',
		});
	});

	it('should not stack the same alert twice while it is pending', () => {
		const event = {
			event: 'hostkey.request',
			data: {
				kind: 'alert',
				alias: 'nova',
				reason: { kind: 'revoked', path: '/k', line: 2 },
				presentedFingerprint: 'SHA256:abc',
			},
		} as const;

		const next = reduceDaemonEvent(
			reduceDaemonEvent(INITIAL_DAEMON_STATE, event),
			event
		);

		expect(next.prompts).toHaveLength(1);
	});

	it('should remove the prompt on prompt.resolved', () => {
		const state = reduceDaemonEvent(
			INITIAL_DAEMON_STATE,
			passwordRequestEvent('p1')
		);

		const next = reduceDaemonEvent(state, {
			event: 'prompt.resolved',
			data: { promptId: 'p1' },
		});

		expect(next.prompts).toEqual([]);
	});

	it('should flag the quit request on quit.requested', () => {
		const next = reduceDaemonEvent(INITIAL_DAEMON_STATE, {
			event: 'quit.requested',
			data: {},
		});

		expect(next.isQuitRequested).toBe(true);
	});
});
