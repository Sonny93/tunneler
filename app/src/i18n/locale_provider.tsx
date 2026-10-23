import { i18n } from '@lingui/core';
import { I18nProvider } from '@lingui/react';
import { useEffect, useState, type ReactNode } from 'react';

import { activateLocale } from '#/i18n/activate_locale';
import { resolveLocale } from '#/i18n/resolve_locale';
import { usePreferredLanguages } from '#/i18n/use_preferred_languages';
import { useDaemonStore } from '#/state/daemon_store';

type LocaleProviderProps = {
	children: ReactNode;
};

/**
 * Keeps Lingui on the locale chosen by `settings.language` (or the OS languages for
 * `system`) and mirrors it on `<html lang>`. Renders nothing until the first catalog
 * is loaded, so no untranslated text flashes.
 */
export function LocaleProvider({ children }: Readonly<LocaleProviderProps>) {
	const language = useDaemonStore(
		(state) => state.settings?.language ?? 'system'
	);
	const preferredLanguages = usePreferredLanguages();
	const [isActivated, setIsActivated] = useState(false);
	const locale = resolveLocale(language, preferredLanguages);

	useEffect(() => {
		let isCurrent = true;
		void activateLocale(locale).then(() => {
			if (!isCurrent) return;
			document.documentElement.lang = locale;
			setIsActivated(true);
		});
		return () => {
			isCurrent = false;
		};
	}, [locale]);

	if (!isActivated) return null;
	return <I18nProvider i18n={i18n}>{children}</I18nProvider>;
}
