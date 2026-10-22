import { msg, t } from '@lingui/core/macro';
import type { Theme } from '@minimalstuff/ui';
import type { I18n, MessageDescriptor } from '@lingui/core';

const THEME_LABELS: Readonly<Record<Theme, MessageDescriptor>> = {
	system: msg`system`,
	light: msg`light`,
	dark: msg`dark`,
};

export function getThemeAriaLabel(theme: Theme, i18n: I18n): string {
	const themeLabel = i18n._(THEME_LABELS[theme]);
	return t(i18n)`Theme: ${themeLabel}, change`;
}
