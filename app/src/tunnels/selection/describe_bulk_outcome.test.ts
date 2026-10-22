import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import {
	describeBulkOutcome,
	type BulkVerb,
} from '#/tunnels/selection/describe_bulk_outcome';

describe('describeBulkOutcome', () => {
	it('should report the plural count in the success tone', () => {
		expect(
			describeBulkOutcome(
				{ total: 3, succeededIds: ['a', 'b', 'c'], failures: [] },
				'start',
				i18n
			)
		).toEqual({ tone: 'success', message: '3 tunnels started' });
	});

	it('should use the singular for one tunnel', () => {
		expect(
			describeBulkOutcome(
				{ total: 1, succeededIds: ['a'], failures: [] },
				'start',
				i18n
			)
		).toEqual({ tone: 'success', message: '1 tunnel started' });
	});

	it.each<[BulkVerb, string]>([
		['stop', '2 tunnels stopped'],
		['restart', '2 tunnels restarted'],
		['standby', '2 tunnels sent to standby'],
		['remove', '2 tunnels deleted'],
		['update', '2 tunnels updated'],
	])('should word the success of the %s action', (verb, message) => {
		expect(
			describeBulkOutcome(
				{ total: 2, succeededIds: ['a', 'b'], failures: [] },
				verb,
				i18n
			).message
		).toBe(message);
	});

	it('should name the failed tunnels in the error tone', () => {
		expect(
			describeBulkOutcome(
				{
					total: 3,
					succeededIds: ['a', 'b'],
					failures: [{ label: '5173 → nova:5173', message: 'port in use' }],
				},
				'start',
				i18n
			)
		).toEqual({
			tone: 'error',
			message: '2 of 3 started; failed: 5173 → nova:5173 (port in use)',
		});
	});

	it('should list every failure', () => {
		expect(
			describeBulkOutcome(
				{
					total: 2,
					succeededIds: [],
					failures: [
						{ label: '5173 → nova:5173', message: 'x' },
						{ label: '8080 → nova:8080', message: 'y' },
					],
				},
				'stop',
				i18n
			).message
		).toBe(
			'0 of 2 stopped; failed: 5173 → nova:5173 (x), 8080 → nova:8080 (y)'
		);
	});
});
