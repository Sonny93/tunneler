const SINGLE_TUNNEL = 1;

export function describeTunnelsToStop(count: number): string {
	if (count === SINGLE_TUNNEL)
		return '1 tunnel ou session X11 en cours sera arrêté.';
	return `${count} tunnels ou sessions X11 en cours seront arrêtés.`;
}
