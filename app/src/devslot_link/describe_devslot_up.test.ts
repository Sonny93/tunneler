import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import { describeDevslotUp } from '#/devslot_link/describe_devslot_up';
import { buildDevslotUpResult } from '#/testing/factories';

describe('describeDevslotUp', () => {
	it('should summarize a clean start without problems', () => {
		const report = describeDevslotUp(buildDevslotUpResult(), i18n);

		expect(report).toEqual({
			summary: 'Opened web-shop-098a on vega',
			problems: [],
		});
	});

	it('should say why nothing opened', () => {
		const report = describeDevslotUp(
			buildDevslotUpResult({ opened: undefined, openError: 'no browser' }),
			i18n
		);

		expect(report.summary).toContain(
			'The browser could not be opened: no browser'
		);
	});

	it('should report failed and blocked ports as problems', () => {
		const report = describeDevslotUp(
			buildDevslotUpResult({
				failed: [{ remotePort: 20051, message: 'refused' }],
				skipped: [
					{ remotePort: 20052, reason: 'localPortBusy' },
					{ remotePort: 20053, reason: 'needsReview' },
					{ remotePort: 20054, reason: 'existingTunnel' },
				],
			}),
			i18n
		);

		expect(report.problems).toHaveLength(4);
		expect(report.problems[3]).toContain(
			'a tunnel you configured already forwards it'
		);
	});

	it('should mention ports below 1024 in the summary only', () => {
		const report = describeDevslotUp(
			buildDevslotUpResult({
				skipped: [{ remotePort: 80, reason: 'privilegedPort' }],
			}),
			i18n
		);

		expect(report.problems).toEqual([]);
		expect(report.summary).toContain('Ports below 1024');
	});
});
