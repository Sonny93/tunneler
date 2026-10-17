import type { FailureReasonDto } from '#/daemon/protocol';

const MILLISECONDS_PER_SECOND = 1000;

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
		case 'agentUnresponsive':
			return `L'agent SSH ne répond pas (aucune réponse en ${Math.round(reason.timeoutMs / MILLISECONDS_PER_SECOND)} s) : attend-il une confirmation ?`;
	}
}
