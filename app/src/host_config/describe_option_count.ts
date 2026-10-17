/** The polite status text for the number of options currently shown. */
export function describeOptionCount(count: number): string {
	if (count === 0) return 'Aucune option affichée';
	return count === 1 ? '1 option affichée' : `${count} options affichées`;
}
