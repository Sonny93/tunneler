import { describe, expect, it } from 'vitest';

import unoConfig from '../uno.config';

function isScanned(path: string): boolean {
	const { pipeline } = unoConfig.content ?? {};
	if (!pipeline) return false;
	const patterns = [pipeline.include ?? []]
		.flat()
		.filter((pattern) => pattern instanceof RegExp);
	return patterns.some((pattern) => pattern.test(path));
}

describe('uno config content pipeline', () => {
	it('should scan plain .ts files so icon classes defined there are generated', () => {
		expect(isScanned('/src/tunnel_details/log_level_appearance.ts')).toBe(true);
	});

	it('should keep scanning .tsx files', () => {
		expect(isScanned('/src/shared/status_badge.tsx')).toBe(true);
	});
});
