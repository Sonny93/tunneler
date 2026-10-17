import { describe, expect, it } from 'vitest';

import {
	describeDetectedCount,
	describeImportAction,
	describeImportedCount,
} from '#/import/import_texts';

describe('describeDetectedCount', () => {
	it.each([
		[0, 'Aucun tunnel détecté'],
		[1, '1 tunnel détecté'],
		[8, '8 tunnels détectés'],
	])('should describe %i detected tunnels', (count, expected) => {
		expect(describeDetectedCount(count)).toBe(expected);
	});
});

describe('describeImportAction', () => {
	it.each([
		[1, 'Importer 1 tunnel'],
		[3, 'Importer 3 tunnels'],
	])('should label the action for %i tunnels', (count, expected) => {
		expect(describeImportAction(count)).toBe(expected);
	});
});

describe('describeImportedCount', () => {
	it.each([
		[1, '1 tunnel importé'],
		[3, '3 tunnels importés'],
	])('should confirm %i imported tunnels', (count, expected) => {
		expect(describeImportedCount(count)).toBe(expected);
	});
});
