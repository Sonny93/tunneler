import type { ReactNode } from 'react';
import { UIProvider, type UILabels } from '@minimalstuff/ui';

import { getThemeAriaLabel } from '#/header/theme_aria_label';

const FRENCH_LABELS: Partial<UILabels> = {
	close: 'Fermer',
	confirm: 'Confirmer',
	cancel: 'Annuler',
	noResults: 'Aucun résultat',
	clearSelection: 'Effacer la sélection',
	search: 'Rechercher',
	clear: 'Effacer',
	required: 'obligatoire',
	currentTheme: getThemeAriaLabel,
	overflowCount: (hiddenCount) => `+${hiddenCount}`,
	selectedCount: (selectedCount) =>
		selectedCount > 1
			? `${selectedCount} sélectionnés`
			: `${selectedCount} sélectionné`,
	maxSelectedHint: (maxSelectedValues) =>
		`Maximum de ${maxSelectedValues} atteint`,
};

/**
 * Provides the French labels of the lib components. The app shows its own error
 * summary and focuses the first invalid field, so field errors are not announced
 * a second time as alerts. Mount `ModalProvider` below it so modals get the labels.
 */
export function AppUIProvider({
	children,
}: Readonly<{ children: ReactNode }>): ReactNode {
	return (
		<UIProvider labels={FRENCH_LABELS} announceFieldErrors={false}>
			{children}
		</UIProvider>
	);
}
