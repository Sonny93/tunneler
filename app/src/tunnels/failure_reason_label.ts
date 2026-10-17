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
		case 'remoteForwardRefused':
			return `Le serveur refuse d'écouter sur ${reason.address}`;
	}
}
