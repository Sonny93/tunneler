import type { Theme } from '@minimalstuff/ui';

const THEME_LABELS: Readonly<Record<Theme, string>> = {
	system: 'système',
	light: 'clair',
	dark: 'sombre',
};

export function getThemeAriaLabel(theme: Theme): string {
	return `Thème : ${THEME_LABELS[theme]}, changer`;
}
