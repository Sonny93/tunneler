import type { TunnelKindDto } from '#/daemon/protocol';

export const TUNNEL_KIND_OPTIONS = [
	{
		value: 'local',
		label: 'Local (-L)',
		description: 'Écoute sur ce poste, cible vue depuis le serveur',
	},
	{
		value: 'remote',
		label: 'Distant (-R)',
		description: 'Écoute sur le serveur, cible vue depuis ce poste',
	},
	{
		value: 'dynamic',
		label: 'Dynamique (-D, SOCKS)',
		description: 'Proxy SOCKS sur ce poste',
	},
] as const satisfies readonly {
	value: TunnelKindDto;
	label: string;
	description: string;
}[];

type TunnelKindFieldTexts = {
	listenPortLabel: string;
	targetHostLabel: string;
	targetHostHint: string;
};

const FIELD_TEXTS: Readonly<Record<TunnelKindDto, TunnelKindFieldTexts>> = {
	local: {
		listenPortLabel: "Port d'écoute",
		targetHostLabel: 'Hôte cible',
		targetHostHint: 'Vu depuis le serveur SSH',
	},
	remote: {
		listenPortLabel: "Port d'écoute sur le serveur",
		targetHostLabel: 'Hôte cible (depuis ce poste)',
		targetHostHint: 'Atteint depuis ce poste',
	},
	dynamic: {
		listenPortLabel: "Port d'écoute",
		targetHostLabel: '',
		targetHostHint: '',
	},
};

export function getTunnelKindFieldTexts(
	kind: TunnelKindDto
): TunnelKindFieldTexts {
	return FIELD_TEXTS[kind];
}
