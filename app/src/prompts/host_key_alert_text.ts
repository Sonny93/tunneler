import type { HostKeyAlertReasonDto } from '#/daemon/protocol';

export type HostKeyAlertFact = { label: string; value: string };

export function getHostKeyAlertTitle(
	reason: Readonly<HostKeyAlertReasonDto>
): string {
	switch (reason.kind) {
		case 'changed':
			return "Clé d'hôte modifiée";
		case 'revoked':
			return "Clé d'hôte révoquée";
		case 'unknownRejectedByPolicy':
			return 'Hôte inconnu refusé';
	}
}

export function getHostKeyAlertFacts(
	reason: Readonly<HostKeyAlertReasonDto>,
	presentedFingerprint: string
): HostKeyAlertFact[] {
	const presented = {
		label: 'Empreinte présentée',
		value: presentedFingerprint,
	};
	switch (reason.kind) {
		case 'changed':
			return [
				presented,
				{ label: 'Empreinte enregistrée', value: reason.knownFingerprint },
				{ label: 'Emplacement', value: `${reason.path}:${reason.line}` },
			];
		case 'revoked':
			return [
				presented,
				{ label: 'Emplacement', value: `${reason.path}:${reason.line}` },
			];
		case 'unknownRejectedByPolicy':
			return [presented];
	}
}

export function getHostKeyAlertAdvice(
	reason: Readonly<HostKeyAlertReasonDto>
): string | null {
	switch (reason.kind) {
		case 'changed':
			return `Si le changement est légitime, supprimez la ligne ${reason.line} de ${reason.path} puis réessayez.`;
		case 'revoked':
			return null;
		case 'unknownRejectedByPolicy':
			return "StrictHostKeyChecking interdit d'accepter une clé inconnue pour cet hôte.";
	}
}
