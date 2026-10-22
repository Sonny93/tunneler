import { describe, expect, it } from 'vitest';

import { describeCutConnections } from '#/standby/describe_cut_connections';

describe('describeCutConnections', () => {
	it('should use the singular when one connection is open', () => {
		expect(describeCutConnections(1)).toBe('1 connexion ouverte sera coupée.');
	});

	it('should use the plural when several connections are open', () => {
		expect(describeCutConnections(2)).toBe(
			'2 connexions ouvertes seront coupées.'
		);
	});
});
