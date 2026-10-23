import { i18n } from '@lingui/core';
import { I18nProvider } from '@lingui/react';
import type { ReactNode } from 'react';

type I18nTestProviderProps = {
	children: ReactNode;
};

/**
 * Provides Lingui with whichever locale is active. `test_setup.ts` activates `en`
 * before each test; a test that needs another locale calls `activateLocale`.
 * Use it as the `wrapper` of `renderHook`, or through `renderWithI18n`.
 */
export const I18nTestProvider = ({
	children,
}: Readonly<I18nTestProviderProps>) => (
	<I18nProvider i18n={i18n}>{children}</I18nProvider>
);
