import type { X11StateDto } from '#/daemon/protocol';

export function getX11StateLabel(state: Readonly<X11StateDto>): string {
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

/** The remote display of an active anchor or the message of a failed one. */
export function getX11StateDetail(state: Readonly<X11StateDto>): string | null {
	switch (state.kind) {
		case 'active':
			return state.display;
		case 'failed':
			return state.message;
		case 'stopped':
		case 'starting':
		case 'reconnecting':
			return null;
	}
}
