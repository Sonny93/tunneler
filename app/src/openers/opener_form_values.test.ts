import { describe, expect, it } from 'vitest';

import {
	parseArguments,
	parsePortList,
	slugifyName,
	toOpenerFormValues,
	toOpenerSpec,
} from '#/openers/opener_form_values';

describe('slugifyName', () => {
	it('should lowercase and join words with dashes', () => {
		expect(slugifyName('Mon Client SQL')).toBe('mon-client-sql');
	});

	it('should drop accents and punctuation', () => {
		expect(slugifyName(' Éditeur (bêta) ! ')).toBe('editeur-beta');
	});

	it('should give nothing for a name without letters or digits', () => {
		expect(slugifyName('---')).toBe('');
	});
});

describe('parseArguments', () => {
	it('should give one argument per line and keep spaces inside them', () => {
		expect(parseArguments('-con\ndriver=pg host={host}')).toEqual([
			'-con',
			'driver=pg host={host}',
		]);
	});

	it('should skip blank lines and accept Windows line ends', () => {
		expect(parseArguments('a\r\n\r\nb\r\n')).toEqual(['a', 'b']);
	});
});

describe('parsePortList', () => {
	it('should read comma-separated ports', () => {
		expect(parsePortList('5432, 3306 ,')).toEqual([5432, 3306]);
	});

	it('should give nothing for blank text', () => {
		expect(parsePortList('  ')).toEqual([]);
	});
});

describe('toOpenerSpec', () => {
	it('should round-trip a spec through the form values', () => {
		const spec = {
			id: 'dbeaver',
			name: 'DBeaver',
			command: 'dbeaver',
			args: ['-con', 'host={host}'],
			ports: [5432, 3306],
		};

		expect(toOpenerSpec(toOpenerFormValues(spec))).toEqual(spec);
	});

	it('should trim the texts', () => {
		const spec = toOpenerSpec({
			id: ' a ',
			name: ' A ',
			command: ' a ',
			args: '',
			ports: '',
		});

		expect([spec.id, spec.name, spec.command]).toEqual(['a', 'A', 'a']);
	});
});
