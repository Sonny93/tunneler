import type { TunnelStateDto } from '#/daemon/protocol';

export function getTunnelStateLabel(state: Readonly<TunnelStateDto>): string {
	switch (state.kind) {
		case 'stopped':
			return 'Arrêté';
		case 'starting':
			return 'Démarrage…';
		case 'active':
			return 'Actif';
		case 'reconnecting':
			return `Reconnexion (tentative ${state.attempt})`;
		case 'failed':
			return 'Échec';
	}
}
