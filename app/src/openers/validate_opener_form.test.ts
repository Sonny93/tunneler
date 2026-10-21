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
		expect(validateOpenerForm({ ...valid, name: ' ' }, []).name).toBe(
			'Le nom est obligatoire.'
		);
	});

	it('should require a command', () => {
		expect(validateOpenerForm({ ...valid, command: '' }, []).command).toBe(
			'La commande est obligatoire.'
		);
	});

	it('should require an identifier', () => {
		expect(validateOpenerForm({ ...valid, id: '' }, []).id).toBe(
			"L'identifiant est obligatoire."
		);
	});

	it('should refuse an identifier with uppercase letters', () => {
		expect(validateOpenerForm({ ...valid, id: 'DBeaver' }, []).id).toBe(
			"L'identifiant ne peut contenir que des minuscules, des chiffres et des tirets."
		);
	});

	it('should refuse the reserved browser identifier', () => {
		expect(validateOpenerForm({ ...valid, id: 'browser' }, []).id).toBe(
			'« browser » est réservé au navigateur.'
		);
	});

	it('should refuse an identifier already taken', () => {
		expect(validateOpenerForm(valid, ['dbeaver']).id).toBe(
			'Cet identifiant est déjà utilisé par une de vos applications.'
		);
	});

	it('should refuse an unknown placeholder', () => {
		expect(
			validateOpenerForm({ ...valid, args: '--db={database}' }, []).args
		).toBe(
			'Paramètre inconnu : {database}. Seuls {host}, {port} et {url} existent.'
		);
	});

	it('should accept the three known placeholders', () => {
		expect(
			validateOpenerForm({ ...valid, args: '{host}\n{port}\n{url}' }, []).args
		).toBeUndefined();
	});

	it('should refuse a port out of range', () => {
		expect(validateOpenerForm({ ...valid, ports: '70000' }, []).ports).toBe(
			'Les ports doivent être des entiers entre 1 et 65535, séparés par des virgules.'
		);
	});

	it('should refuse a port that is not a number', () => {
		expect(validateOpenerForm({ ...valid, ports: '80, web' }, []).ports).toBe(
			'Les ports doivent être des entiers entre 1 et 65535, séparés par des virgules.'
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
