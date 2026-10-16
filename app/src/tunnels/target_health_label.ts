import type { TargetHealthDto } from '#/daemon/protocol';

export function getTargetHealthLabel(
	health: Readonly<TargetHealthDto>
): string {
	switch (health.kind) {
		case 'unknown':
			return 'Santé inconnue';
		case 'reachable':
			return 'Cible joignable';
		case 'unreachable':
			return 'Cible injoignable';
	}
}

/** The last error of an unreachable target, `null` for the other health kinds. */
export function getTargetHealthError(
	health: Readonly<TargetHealthDto>
): string | null {
	return health.kind === 'unreachable' ? health.error : null;
}
