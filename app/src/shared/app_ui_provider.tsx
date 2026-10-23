import type { I18n } from '@lingui/core';
import { plural, msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { UIProvider, type UILabels } from '@minimalstuff/ui';
import type { ReactNode } from 'react';

import { getThemeAriaLabel } from '#/header/theme_aria_label';

function buildLabels(i18n: I18n): Partial<UILabels> {
	return {
		close: i18n._(msg`Close`),
		confirm: i18n._(msg`Confirm`),
		cancel: i18n._(msg`Cancel`),
		noResults: i18n._(msg`No results`),
		clearSelection: i18n._(msg`Clear selection`),
		search: i18n._(msg`Search`),
		clear: i18n._(msg`Clear`),
		required: i18n._(msg`required`),
		currentTheme: (theme) => getThemeAriaLabel(theme, i18n),
		overflowCount: (hiddenCount) => `+${hiddenCount}`,
		selectedCount: (selectedCount) =>
			i18n._(
				msg`${plural(selectedCount, { one: '# selected', other: '# selected' })}`
			),
		maxSelectedHint: (maxSelectedValues) =>
			i18n._(msg`Maximum of ${maxSelectedValues} reached`),
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
