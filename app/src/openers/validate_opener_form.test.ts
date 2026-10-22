import { i18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';
import { describe, expect, it } from 'vitest';

import {
	getFirstInvalidOpenerField,
	validateOpenerForm,
} from '#/openers/validate_opener_form';

const valid = {
	name: 'DBeaver',
	id: 'dbeaver',
	command: 'dbeaver',
	args: '-con\nhost={host}',
	ports: '5432, 3306',
};

describe('validateOpenerForm', () => {
	it('should accept valid values', () => {
		expect(validateOpenerForm(valid, [])).toEqual({});
	});

	it('should require a name', () => {
		expect(validateOpenerForm({ ...valid, name: ' ' }, []).name).toEqual(
			msg`The name is required.`
		);
	});

	it('should require a command', () => {
		expect(validateOpenerForm({ ...valid, command: '' }, []).command).toEqual(
			msg`The command is required.`
		);
	});

	it('should require an identifier', () => {
		expect(validateOpenerForm({ ...valid, id: '' }, []).id).toEqual(
			msg`The identifier is required.`
		);
	});

	it('should refuse an identifier with uppercase letters', () => {
		expect(validateOpenerForm({ ...valid, id: 'DBeaver' }, []).id).toEqual(
			msg`The identifier can only contain lowercase letters, digits and dashes.`
		);
	});

	it('should refuse the reserved browser identifier', () => {
		expect(validateOpenerForm({ ...valid, id: 'browser' }, []).id).toEqual(
			msg`"browser" is reserved for the browser.`
		);
	});

	it('should refuse an identifier already taken', () => {
		expect(validateOpenerForm(valid, ['dbeaver']).id).toEqual(
			msg`This identifier is already used by one of your applications.`
		);
	});

	it('should refuse an unknown placeholder', () => {
		const { args } = validateOpenerForm(
			{ ...valid, args: '--db={database}' },
			[]
		);

		expect(args && i18n._(args)).toBe(
			'Unknown parameter: {database}. Only {host}, {port} and {url} exist.'
		);
	});

	it('should accept the three known placeholders', () => {
		expect(
			validateOpenerForm({ ...valid, args: '{host}\n{port}\n{url}' }, []).args
		).toBeUndefined();
	});

	it('should refuse a port out of range', () => {
		expect(validateOpenerForm({ ...valid, ports: '70000' }, []).ports).toEqual(
			msg`The ports must be whole numbers between 1 and 65535, separated by commas.`
		);
	});

	it('should refuse a port that is not a number', () => {
		expect(
			validateOpenerForm({ ...valid, ports: '80, web' }, []).ports
		).toEqual(
			msg`The ports must be whole numbers between 1 and 65535, separated by commas.`
		);
	});

	it('should accept blank ports', () => {
		expect(
			validateOpenerForm({ ...valid, ports: '' }, []).ports
		).toBeUndefined();
	});

	it('should find the first invalid field in form order', () => {
		const errors = validateOpenerForm(
			{ ...valid, command: '', ports: 'x' },
			[]
		);

		expect(getFirstInvalidOpenerField(errors)).toBe('command');
	});
});
