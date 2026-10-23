import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

export type GlobalShortcutKey = 'n' | 'i' | ',' | 'b' | '?' | 'F1';

export type ShortcutDefinition = {
	keys: readonly string[];
	description: string;
};

export function getGlobalShortcuts(i18n: I18n): readonly ShortcutDefinition[] {
	return [
		{ keys: ['n'], description: i18n._(msg`New tunnel`) },
		{ keys: ['i'], description: i18n._(msg`Import tunnels`) },
		{ keys: [','], description: i18n._(msg`Open settings`) },
		{ keys: ['b'], description: i18n._(msg`Hide or show the sidebar`) },
		{ keys: ['?'], description: i18n._(msg`Show keyboard shortcuts`) },
		{ keys: ['F1'], description: i18n._(msg`Open the documentation`) },
		{ keys: ['Ctrl', 'K'], description: i18n._(msg`Open the command palette`) },
	];
}

export function getListShortcuts(i18n: I18n): readonly ShortcutDefinition[] {
	return [
		{ keys: ['↑', '↓'], description: i18n._(msg`Previous / next tunnel`) },
		{
			keys: [i18n._(msg`Home`), i18n._(msg`End`)],
			description: i18n._(msg`First / last tunnel`),
		},
		{
			keys: ['←', '→'],
			description: i18n._(msg`Checkbox, switch and menu of the row`),
		},
		{
			keys: [i18n._(msg`Space`)],
			description: i18n._(
				msg`Start or stop the tunnel, check or uncheck the checkbox`
			),
		},
		{
			keys: [i18n._(msg`Shift`), i18n._(msg`click`)],
			description: i18n._(msg`Select a range of tunnels`),
		},
		{
			keys: ['Ctrl', 'A'],
			description: i18n._(msg`Select all displayed tunnels`),
		},
		{ keys: [i18n._(msg`Esc`)], description: i18n._(msg`Clear the selection`) },
		{
			keys: [i18n._(msg`Esc`)],
			description: i18n._(msg`Close the panel or the dialog`),
		},
	];
}
