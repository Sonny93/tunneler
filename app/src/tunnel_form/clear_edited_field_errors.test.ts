import { msg } from '@lingui/core/macro';
import { describe, expect, it } from 'vitest';

import { toTunnelFormValues } from '#/tunnel_form/tunnel_form_values';
import { clearEditedFieldErrors } from '#/tunnel_form/clear_edited_field_errors';

const ALL_ERRORS = {
	host: msg`host error`,
	tags: msg`tags error`,
	listenPort: msg`listen error`,
	targetPort: msg`target error`,
};

const linkedValues = toTunnelFormValues(null);
const unlinkedValues = { ...linkedValues, isTargetPortLinked: false };

describe('clearEditedFieldErrors', () => {
	it('should clear the error of the edited text field', () => {
		expect(
			clearEditedFieldErrors(
				ALL_ERRORS,
				{ type: 'textChanged', field: 'host', value: 'a' },
				linkedValues
			).host
		).toBeUndefined();
	});

	it('should keep the errors of the other fields when a text field is edited', () => {
		expect(
			clearEditedFieldErrors(
				ALL_ERRORS,
				{ type: 'textChanged', field: 'host', value: 'a' },
				linkedValues
			)
		).toEqual({
			tags: ALL_ERRORS.tags,
			listenPort: ALL_ERRORS.listenPort,
			targetPort: ALL_ERRORS.targetPort,
		});
	});

	it('should clear the target port error when the listen port is edited while linked', () => {
		expect(
			clearEditedFieldErrors(
				ALL_ERRORS,
				{ type: 'textChanged', field: 'listenPort', value: '80' },
				linkedValues
			).targetPort
		).toBeUndefined();
	});

	it('should keep the target port error when the listen port is edited while unlinked', () => {
		expect(
			clearEditedFieldErrors(
				ALL_ERRORS,
				{ type: 'textChanged', field: 'listenPort', value: '80' },
				unlinkedValues
			).targetPort
		).toBe(ALL_ERRORS.targetPort);
	});

	it('should clear the target port error when the target port is edited', () => {
		expect(
			clearEditedFieldErrors(
				ALL_ERRORS,
				{ type: 'targetPortChanged', value: '80' },
				unlinkedValues
			).targetPort
		).toBeUndefined();
	});

	it('should clear the target port error when the target port is relinked', () => {
		expect(
			clearEditedFieldErrors(
				ALL_ERRORS,
				{ type: 'targetPortRelinked' },
				unlinkedValues
			).targetPort
		).toBeUndefined();
	});

	it('should clear the target port error when the kind changes', () => {
		expect(
			clearEditedFieldErrors(
				ALL_ERRORS,
				{ type: 'kindChanged', kind: 'dynamic' },
				linkedValues
			).targetPort
		).toBeUndefined();
	});
});
