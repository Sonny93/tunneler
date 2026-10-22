import type { ReactNode } from 'react';
import type { I18n } from '@lingui/core';
import { t, plural } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { UIProvider, type UILabels } from '@minimalstuff/ui';

import { getThemeAriaLabel } from '#/header/theme_aria_label';

function buildLabels(i18n: I18n): Partial<UILabels> {
	return {
		close: t(i18n)`Close`,
		confirm: t(i18n)`Confirm`,
		cancel: t(i18n)`Cancel`,
		noResults: t(i18n)`No results`,
		clearSelection: t(i18n)`Clear selection`,
		search: t(i18n)`Search`,
		clear: t(i18n)`Clear`,
		required: t(i18n)`required`,
		currentTheme: (theme) => getThemeAriaLabel(theme, i18n),
		overflowCount: (hiddenCount) => `+${hiddenCount}`,
		selectedCount: (selectedCount) =>
			t(
				i18n
			)`${plural(selectedCount, { one: '# selected', other: '# selected' })}`,
		maxSelectedHint: (maxSelectedValues) =>
			t(i18n)`Maximum of ${maxSelectedValues} reached`,
	};
}

/**
 * Provides the labels of the lib components in the active language. The app shows
 * its own error summary and focuses the first invalid field, so field errors are
 * not announced a second time as alerts. Mount `ModalProvider` below it so modals
 * get the labels.
 */
export function AppUIProvider({
	children,
}: Readonly<{ children: ReactNode }>): ReactNode {
	const { i18n } = useLingui();

	return (
		<UIProvider labels={buildLabels(i18n)} announceFieldErrors={false}>
			{children}
		</UIProvider>
	);
}
