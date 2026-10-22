import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';

export type GlobalShortcutKey = 'n' | 'i' | ',' | 'b' | '?';

export type ShortcutDefinition = {
	keys: readonly string[];
	description: string;
};

export function getGlobalShortcuts(i18n: I18n): readonly ShortcutDefinition[] {
	return [
		{ keys: ['n'], description: t(i18n)`New tunnel` },
		{ keys: ['i'], description: t(i18n)`Import tunnels` },
		{ keys: [','], description: t(i18n)`Open settings` },
		{ keys: ['b'], description: t(i18n)`Hide or show the sidebar` },
		{ keys: ['?'], description: t(i18n)`Show keyboard shortcuts` },
		{ keys: ['Ctrl', 'K'], description: t(i18n)`Open the command palette` },
	];
}

export function getListShortcuts(i18n: I18n): readonly ShortcutDefinition[] {
	return [
		{ keys: ['↑', '↓'], description: t(i18n)`Previous / next tunnel` },
		{
			keys: [t(i18n)`Home`, t(i18n)`End`],
			description: t(i18n)`First / last tunnel`,
		},
		{
			keys: ['←', '→'],
			description: t(i18n)`Checkbox, switch and menu of the row`,
		},
		{
			keys: [t(i18n)`Space`],
			description: t(
				i18n
			)`Start or stop the tunnel, check or uncheck the checkbox`,
		},
		{
			keys: [t(i18n)`Shift`, t(i18n)`click`],
			description: t(i18n)`Select a range of tunnels`,
		},
		{
			keys: ['Ctrl', 'A'],
			description: t(i18n)`Select all displayed tunnels`,
		},
		{ keys: [t(i18n)`Esc`], description: t(i18n)`Clear the selection` },
		{
			keys: [t(i18n)`Esc`],
			description: t(i18n)`Close the panel or the dialog`,
		},
	];
}
