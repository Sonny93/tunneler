import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import {
	getTargetHealthError,
	getTargetHealthLabel,
} from '#/tunnels/target_health_label';

describe('getTargetHealthLabel', () => {
	it.each([
		[{ kind: 'unknown' }, 'Health unknown'],
		[{ kind: 'reachable' }, 'Target reachable'],
		[{ kind: 'unreachable', since: 0, error: 'refused' }, 'Target unreachable'],
	] as const)('should label %j as %s', (health, label) => {
		expect(getTargetHealthLabel(health, i18n)).toBe(label);
	});
});

describe('getTargetHealthError', () => {
	it('should return the error of an unreachable target', () => {
		expect(
			getTargetHealthError({ kind: 'unreachable', since: 0, error: 'refused' })
		).toBe('refused');
	});

	it('should return null when the target is reachable', () => {
		expect(getTargetHealthError({ kind: 'reachable' })).toBeNull();
	});
});
