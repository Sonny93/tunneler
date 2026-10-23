import { describe, expect, it } from 'vitest';

import { buildX11Anchor } from '#/testing/factories';
import { findX11State } from '#/x11/find_x11_state';

describe('findX11State', () => {
	it('should give the state of the anchor of the host', () => {
		const anchors = [buildX11Anchor({ host: 'other' }), buildX11Anchor()];

		expect(findX11State(anchors, 'nova')).toEqual({
			kind: 'active',
			display: 'localhost:10.0',
		});
	});

	it('should give stopped when the host has no anchor', () => {
		expect(findX11State([], 'nova')).toEqual({ kind: 'stopped' });
	});
});
