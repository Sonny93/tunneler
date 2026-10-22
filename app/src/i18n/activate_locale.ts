import { i18n } from '@lingui/core';

import type { Locale } from '#/i18n/supported_locales';

/** Loads the catalog of `locale` on demand and makes it the active one. */
export async function activateLocale(locale: Locale): Promise<void> {
	const { messages } = await import(`../locales/${locale}/messages.po`);
	i18n.loadAndActivate({ locale, messages });
}
