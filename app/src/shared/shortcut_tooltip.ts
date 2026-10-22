/** The tooltip text of a control: its name, followed by its keyboard shortcut, e.g. `Settings (,)`. */
export function formatShortcutTooltip(
	label: string,
	shortcutHint: string
): string {
	return `${label} (${shortcutHint})`;
}
