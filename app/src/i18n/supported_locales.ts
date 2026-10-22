export const SUPPORTED_LOCALES = ['en', 'fr'] as const;

export type Locale = (typeof SUPPORTED_LOCALES)[number];

export const SOURCE_LOCALE: Locale = 'en';

export function isSupportedLocale(candidate: string): candidate is Locale {
	return SUPPORTED_LOCALES.some((locale) => locale === candidate);
}
