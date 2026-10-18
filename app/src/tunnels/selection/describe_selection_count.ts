const SINGLE_TUNNEL_COUNT = 1;

/** « 1 tunnel », « 3 tunnels ». */
export function formatTunnelCount(count: number): string {
	return `${count} ${count > SINGLE_TUNNEL_COUNT ? 'tunnels' : 'tunnel'}`;
}

/** « Aucun tunnel sélectionné », « 1 tunnel sélectionné », « 3 tunnels sélectionnés ». */
export function describeSelectionCount(count: number): string {
	if (count === 0) return 'Aucun tunnel sélectionné';
	if (count === SINGLE_TUNNEL_COUNT) return '1 tunnel sélectionné';
	return `${count} tunnels sélectionnés`;
}
