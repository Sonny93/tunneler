import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeCutConnections } from '#/standby/describe_cut_connections';

describe('describeCutConnections', () => {
	it('should use the singular when one connection is open', () => {
		expect(describeCutConnections(1, i18n)).toBe(
			'1 open connection will be cut.'
		);
	});

	it('should use the plural when several connections are open', () => {
		expect(describeCutConnections(2, i18n)).toBe(
			'2 open connections will be cut.'
		);
	});
});
