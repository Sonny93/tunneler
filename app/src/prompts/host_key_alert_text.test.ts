import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import type { HostKeyAlertReasonDto } from '#/daemon/protocol';
import {
	getHostKeyAlertAdvice,
	getHostKeyAlertFacts,
	getHostKeyAlertTitle,
} from '#/prompts/host_key_alert_text';

const changed: HostKeyAlertReasonDto = {
	kind: 'changed',
	knownFingerprint: 'SHA256:known',
	path: '/home/jules/.ssh/known_hosts',
	line: 12,
};
const revoked: HostKeyAlertReasonDto = {
	kind: 'revoked',
	path: '/home/jules/.ssh/known_hosts',
	line: 3,
};
const policy: HostKeyAlertReasonDto = { kind: 'unknownRejectedByPolicy' };

describe('host key alert text', () => {
	it.each([
		[changed, 'Host key changed'],
		[revoked, 'Host key revoked'],
		[policy, 'Unknown host rejected'],
	])('should title %o', (reason, title) => {
		expect(getHostKeyAlertTitle(reason, i18n)).toBe(title);
	});

	it('should list presented, stored fingerprints and location for a changed key', () => {
		expect(getHostKeyAlertFacts(changed, 'SHA256:new', i18n)).toEqual([
			{ label: 'Presented fingerprint', value: 'SHA256:new' },
			{ label: 'Stored fingerprint', value: 'SHA256:known' },
			{ label: 'Location', value: '/home/jules/.ssh/known_hosts:12' },
		]);
	});

	it('should list the presented fingerprint and location for a revoked key', () => {
		expect(
			getHostKeyAlertFacts(revoked, 'SHA256:new', i18n).map(
				(fact) => fact.label
			)
		).toEqual(['Presented fingerprint', 'Location']);
	});

	it('should list only the presented fingerprint under the policy', () => {
		expect(getHostKeyAlertFacts(policy, 'SHA256:new', i18n)).toHaveLength(1);
	});

	it('should tell how to remove a legitimately changed key', () => {
		expect(getHostKeyAlertAdvice(changed, i18n)).toBe(
			'If the change is legitimate, delete line 12 of /home/jules/.ssh/known_hosts and try again.'
		);
	});

	it('should explain the policy refusal', () => {
		expect(getHostKeyAlertAdvice(policy, i18n)).toBe(
			'StrictHostKeyChecking forbids accepting an unknown key for this host.'
		);
	});

	it('should give no advice for a revoked key', () => {
		expect(getHostKeyAlertAdvice(revoked, i18n)).toBeNull();
	});
});
