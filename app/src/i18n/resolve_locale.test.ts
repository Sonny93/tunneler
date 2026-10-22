import { describe, expect, it } from 'vitest';

import { resolveLocale } from '#/i18n/resolve_locale';

describe('resolveLocale', () => {
	it.each([
		['en', ['fr-FR'], 'en'],
		['fr', ['en-US'], 'fr'],
	] as const)(
		'should use the explicit %s choice over the system languages',
		(setting, preferredLanguages, expected) => {
			expect(resolveLocale(setting, preferredLanguages)).toBe(expected);
		}
	);

	it.each([
		[['fr'], 'fr'],
		[['fr-CA', 'en-US'], 'fr'],
		[['FR-be'], 'fr'],
		[['en-US', 'fr-FR'], 'en'],
		[['de-DE', 'fr-FR'], 'fr'],
	] as const)(
		'should follow the system languages %j when the setting is system',
		(preferredLanguages, expected) => {
			expect(resolveLocale('system', preferredLanguages)).toBe(expected);
		}
	);

	it('should fall back to English when no system language is supported', () => {
		expect(resolveLocale('system', ['de-DE', 'ja'])).toBe('en');
	});

	it('should fall back to English when the system has no language', () => {
		expect(resolveLocale('system', [])).toBe('en');
	});
});
