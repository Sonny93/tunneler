import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import {
	describeDetectedCount,
	describeImportAction,
	describeImportedCount,
} from '#/import/import_texts';

describe('describeDetectedCount', () => {
	it.each([
		[0, 'No tunnel detected'],
		[1, '1 tunnel detected'],
		[8, '8 tunnels detected'],
	])('should describe %i detected tunnels', (count, expected) => {
		expect(describeDetectedCount(count, i18n)).toBe(expected);
	});
});

describe('describeImportAction', () => {
	it.each([
		[1, 'Import 1 tunnel'],
		[3, 'Import 3 tunnels'],
	])('should label the action for %i tunnels', (count, expected) => {
		expect(describeImportAction(count, i18n)).toBe(expected);
	});
});

describe('describeImportedCount', () => {
	it.each([
		[1, '1 tunnel imported'],
		[3, '3 tunnels imported'],
	])('should confirm %i imported tunnels', (count, expected) => {
		expect(describeImportedCount(count, i18n)).toBe(expected);
	});
});
