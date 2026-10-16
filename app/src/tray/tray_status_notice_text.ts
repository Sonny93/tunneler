import type { TrayStatusDto } from '#/daemon/protocol';
import type { TrayProblemKind } from '#/state/ui_store';

export type TrayProblem = Extract<TrayStatusDto, { kind: TrayProblemKind }>;

export type TrayStatusNoticeText = { heading: string; body: string };

export function isTrayProblem(
	status: Readonly<TrayStatusDto>
): status is TrayProblem {
	return status.kind === 'missingHost' || status.kind === 'failed';
}

export function describeTrayProblem(
	problem: Readonly<TrayProblem>
): TrayStatusNoticeText {
	if (problem.kind === 'missingHost') {
		return {
			heading: 'Aucune zone de notification détectée',
			body: 'Tunneler continue de tourner quand vous fermez cette fenêtre. Pour la rouvrir, relancez « tunneler ». Sous GNOME, installez l’extension « AppIndicator and KStatusNotifierItem Support » pour afficher l’icône.',
		};
	}
	return {
		heading: 'Icône de notification indisponible',
		body: `L’icône n’a pas pu être créée : ${problem.message}. Tunneler continue de tourner quand vous fermez cette fenêtre ; relancez « tunneler » pour la rouvrir.`,
	};
}
