import type { LanguageSettingDto } from '#/daemon/protocol';
import {
	isSupportedLocale,
	SOURCE_LOCALE,
	type Locale,
} from '#/i18n/supported_locales';

const SYSTEM_LANGUAGE_SETTING = 'system' satisfies LanguageSettingDto;

function toBaseLanguage(languageTag: string): string {
	return languageTag.split('-')[0].toLowerCase();
}

function findPreferredLocale(
	preferredLanguages: readonly string[]
): Locale | undefined {
	return preferredLanguages.map(toBaseLanguage).find(isSupportedLocale);
}

/**
 * The display locale for the `language` setting. An explicit choice wins; `system`
 * takes the first supported language of the OS preference list, English otherwise.
 */
export function resolveLocale(
	setting: LanguageSettingDto,
	preferredLanguages: readonly string[]
): Locale {
	if (setting !== SYSTEM_LANGUAGE_SETTING) return setting;
	return findPreferredLocale(preferredLanguages) ?? SOURCE_LOCALE;
}
