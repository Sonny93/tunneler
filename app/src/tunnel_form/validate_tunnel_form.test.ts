import { describe, expect, it } from 'vitest';

import { EMPTY_TUNNEL_FORM_VALUES } from '#/tunnel_form/tunnel_form_values';
import {
	getFirstInvalidField,
	validateTunnelForm,
} from '#/tunnel_form/validate_tunnel_form';

const valid = {
	...EMPTY_TUNNEL_FORM_VALUES,
	name: 'Adonis',
	host: 'nova',
	listenPort: '3333',
};

describe('validateTunnelForm', () => {
	it('should accept valid values', () => {
		expect(validateTunnelForm(valid)).toEqual({});
	});

	it('should require a name', () => {
		expect(validateTunnelForm({ ...valid, name: '  ' }).name).toBe(
			'Le nom est obligatoire.'
		);
	});

	it('should require a host', () => {
		expect(validateTunnelForm({ ...valid, host: '' }).host).toBe(
			"L'hôte est obligatoire."
		);
	});

	it('should reject a listen port out of range', () => {
		expect(
			validateTunnelForm({ ...valid, listenPort: '70000' }).listenPort
		).toBe('Le port doit être un entier entre 1 et 65535.');
	});

	it('should not validate the target port while linked', () => {
		expect(
			validateTunnelForm({ ...valid, targetPort: 'abc' }).targetPort
		).toBeUndefined();
	});

	it('should validate the target port once unlinked', () => {
		const errors = validateTunnelForm({
			...valid,
			targetPort: '',
			isTargetPortLinked: false,
		});

		expect(errors.targetPort).toBe(
			'Le port doit être un entier entre 1 et 65535.'
		);
	});

	it('should not validate the target port of a dynamic tunnel', () => {
		const errors = validateTunnelForm({
			...valid,
			kind: 'dynamic',
			targetPort: 'abc',
			isTargetPortLinked: false,
		});

		expect(errors.targetPort).toBeUndefined();
	});
});

describe('getFirstInvalidField', () => {
	it('should return the first invalid field in form order', () => {
		expect(getFirstInvalidField({ listenPort: 'x', host: 'y' })).toBe('host');
	});

	it('should return null without errors', () => {
		expect(getFirstInvalidField({})).toBeNull();
	});
});
