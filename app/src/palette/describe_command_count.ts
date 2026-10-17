const SINGULAR_COUNT = 1;

export function describeCommandCount(count: number): string {
	if (count === 0) return 'Aucune commande';
	return count === SINGULAR_COUNT ? '1 commande' : `${count} commandes`;
}
