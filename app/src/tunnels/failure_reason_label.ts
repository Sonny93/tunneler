import type { FailureReasonDto } from '#/daemon/protocol';

export function getFailureReasonLabel(
	reason: Readonly<FailureReasonDto>
): string {
	switch (reason.kind) {
		case 'portInUse':
			return `Port déjà utilisé (${reason.address})`;
		case 'listenFailed':
			return `Écoute impossible sur ${reason.address} : ${reason.message}`;
		case 'connect':
			return `Connexion SSH impossible : ${reason.message}`;
		case 'unsupportedKind':
			return `Type de tunnel non pris en charge (${reason.tunnelKind})`;
	}
}
