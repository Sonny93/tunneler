import { describe, expect, it } from 'vitest';

import { getX11StateDetail, getX11StateLabel } from '#/x11/x11_state_label';

describe('getX11StateLabel', () => {
	it.each([
		[{ kind: 'stopped' }, 'Arrêté'],
		[{ kind: 'starting' }, 'Démarrage…'],
		[{ kind: 'active', display: 'localhost:10.0' }, 'Actif'],
		[
			{ kind: 'reconnecting', attempt: 2, nextRetryAt: 0 },
			'Reconnexion (tentative 2)',
		],
		[{ kind: 'failed', message: 'refusé' }, 'Échec'],
	] as const)('should label %j as %s', (state, label) => {
		expect(getX11StateLabel(state)).toBe(label);
	});
});

describe('getX11StateDetail', () => {
	it.each([
		[{ kind: 'active', display: 'localhost:10.0' }, 'localhost:10.0'],
		[{ kind: 'failed', message: 'refusé' }, 'refusé'],
		[{ kind: 'starting' }, null],
		[{ kind: 'stopped' }, null],
		[{ kind: 'reconnecting', attempt: 1, nextRetryAt: 0 }, null],
	] as const)('should give the detail of %j', (state, detail) => {
		expect(getX11StateDetail(state)).toBe(detail);
	});
});
