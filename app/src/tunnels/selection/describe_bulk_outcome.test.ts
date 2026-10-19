import { describe, expect, it } from 'vitest';

import { describeBulkOutcome } from '#/tunnels/selection/describe_bulk_outcome';

const STARTED = { singular: 'démarré', plural: 'démarrés' };

describe('describeBulkOutcome', () => {
	it('should report the plural count in the success tone', () => {
		expect(
			describeBulkOutcome(
				{ total: 3, succeededIds: ['a', 'b', 'c'], failures: [] },
				STARTED
			)
		).toEqual({ tone: 'success', message: '3 tunnels démarrés' });
	});

	it('should use the singular for one tunnel', () => {
		expect(
			describeBulkOutcome(
				{ total: 1, succeededIds: ['a'], failures: [] },
				STARTED
			)
		).toEqual({ tone: 'success', message: '1 tunnel démarré' });
	});

	it('should name the failed tunnels in the error tone', () => {
		expect(
			describeBulkOutcome(
				{
					total: 3,
					succeededIds: ['a', 'b'],
					failures: [{ label: '5173 → nova:5173', message: 'port occupé' }],
				},
				STARTED
			)
		).toEqual({
			tone: 'error',
			message: '2 sur 3 démarrés ; échec : 5173 → nova:5173 (port occupé)',
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
				STARTED
			).message
		).toBe(
			'0 sur 2 démarrés ; échec : 5173 → nova:5173 (x), 8080 → nova:8080 (y)'
		);
	});
});
