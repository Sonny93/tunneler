import { createGenerator } from 'unocss';
import { describe, expect, it } from 'vitest';

// oxlint-disable-next-line no-restricted-imports -- uno.config.ts sits at the app root, outside the #/ alias
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

	it.each(['scroll-pb-28', 'list-decimal', 'text-tn-danger'])(
		'should generate CSS for %s',
		async (utility) => {
			const generator = await createGenerator(unoConfig);

			const { css } = await generator.generate(utility);

			expect(css).toContain(`.${utility}`);
		}
	);
});
