const SINGLE_TUNNEL = 1;

export function describeTunnelsToStop(count: number): string {
	if (count === SINGLE_TUNNEL) return '1 tunnel actif sera arrêté.';
	return `${count} tunnels actifs seront arrêtés.`;
}
