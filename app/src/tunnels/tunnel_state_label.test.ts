import { describe, expect, it } from 'vitest';

import { getTunnelStateLabel } from '#/tunnels/tunnel_state_label';

describe('getTunnelStateLabel', () => {
	it.each([
		[{ kind: 'stopped' }, 'Arrêté'],
		[{ kind: 'starting' }, 'Démarrage…'],
		[{ kind: 'active' }, 'Actif'],
		[
			{ kind: 'reconnecting', attempt: 2, nextRetryAt: 0 },
			'Reconnexion (tentative 2)',
		],
		[{ kind: 'failed', reason: { kind: 'connect', message: 'x' } }, 'Échec'],
	] as const)('should label %j as %s', (state, label) => {
		expect(getTunnelStateLabel(state)).toBe(label);
	});
});
