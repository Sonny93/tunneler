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
		[changed, "Clé d'hôte modifiée"],
		[revoked, "Clé d'hôte révoquée"],
		[policy, 'Hôte inconnu refusé'],
	])('should title %o', (reason, title) => {
		expect(getHostKeyAlertTitle(reason)).toBe(title);
	});

	it('should list presented, recorded fingerprints and location for a changed key', () => {
		expect(getHostKeyAlertFacts(changed, 'SHA256:new')).toEqual([
			{ label: 'Empreinte présentée', value: 'SHA256:new' },
			{ label: 'Empreinte enregistrée', value: 'SHA256:known' },
			{ label: 'Emplacement', value: '/home/jules/.ssh/known_hosts:12' },
		]);
	});

	it('should list the presented fingerprint and location for a revoked key', () => {
		expect(
			getHostKeyAlertFacts(revoked, 'SHA256:new').map((fact) => fact.label)
		).toEqual(['Empreinte présentée', 'Emplacement']);
	});

	it('should list only the presented fingerprint under the policy', () => {
		expect(getHostKeyAlertFacts(policy, 'SHA256:new')).toHaveLength(1);
	});

	it('should tell how to remove a legitimately changed key', () => {
		expect(getHostKeyAlertAdvice(changed)).toBe(
			'Si le changement est légitime, supprimez la ligne 12 de /home/jules/.ssh/known_hosts puis réessayez.'
		);
	});

	it('should explain the policy refusal', () => {
		expect(getHostKeyAlertAdvice(policy)).toBe(
			"StrictHostKeyChecking interdit d'accepter une clé inconnue pour cet hôte."
		);
	});

	it('should give no advice for a revoked key', () => {
		expect(getHostKeyAlertAdvice(revoked)).toBeNull();
	});
});
