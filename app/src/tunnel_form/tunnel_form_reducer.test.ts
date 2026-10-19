import { describe, expect, it } from 'vitest';

import { tunnelFormReducer } from '#/tunnel_form/tunnel_form_reducer';
import {
	EMPTY_TUNNEL_FORM_VALUES,
	type TunnelFormValues,
} from '#/tunnel_form/tunnel_form_values';

const unlinked: TunnelFormValues = {
	...EMPTY_TUNNEL_FORM_VALUES,
	listenPort: '3333',
	targetPort: '5432',
	isTargetPortLinked: false,
};

describe('tunnelFormReducer', () => {
	it('should make the target port follow the listen port while linked', () => {
		const values = tunnelFormReducer(EMPTY_TUNNEL_FORM_VALUES, {
			type: 'textChanged',
			field: 'listenPort',
			value: '8080',
		});

		expect(values.targetPort).toBe('8080');
	});

	it('should keep the target port when the listen port changes while unlinked', () => {
		const values = tunnelFormReducer(unlinked, {
			type: 'textChanged',
			field: 'listenPort',
			value: '8080',
		});

		expect(values.targetPort).toBe('5432');
	});

	it('should unlink when the target port is edited', () => {
		const values = tunnelFormReducer(EMPTY_TUNNEL_FORM_VALUES, {
			type: 'targetPortChanged',
			value: '22',
		});

		expect([values.targetPort, values.isTargetPortLinked]).toEqual([
			'22',
			false,
		]);
	});

	it('should snap the target port back to the listen port when relinked', () => {
		const values = tunnelFormReducer(unlinked, { type: 'targetPortRelinked' });

		expect([values.targetPort, values.isTargetPortLinked]).toEqual([
			'3333',
			true,
		]);
	});

	it('should change the kind keeping the target values', () => {
		const values = tunnelFormReducer(unlinked, {
			type: 'kindChanged',
			kind: 'dynamic',
		});

		expect([values.kind, values.targetPort, values.isTargetPortLinked]).toEqual(
			['dynamic', '5432', false]
		);
	});

	it('should update any other text field without touching the target port', () => {
		const values = tunnelFormReducer(unlinked, {
			type: 'textChanged',
			field: 'host',
			value: 'nova',
		});

		expect([values.host, values.targetPort]).toEqual(['nova', '5432']);
	});
});
