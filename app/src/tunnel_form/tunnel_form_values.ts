import type { TunnelSpecDto } from '#/daemon/protocol';

export type TunnelFormValues = {
	name: string;
	host: string;
	listenPort: string;
	targetPort: string;
	isTargetPortLinked: boolean;
	targetHost: string;
	listenAddress: string;
	tags: string;
};

export const EMPTY_TUNNEL_FORM_VALUES: TunnelFormValues = {
	name: '',
	host: '',
	listenPort: '',
	targetPort: '',
	isTargetPortLinked: true,
	targetHost: '',
	listenAddress: '',
	tags: '',
};

const TAGS_SEPARATOR = ', ';

/** Form values of an existing tunnel or of a draft, or the empty form when creating. */
export function toTunnelFormValues(
	definition: Readonly<TunnelSpecDto> | null
): TunnelFormValues {
	if (definition === null) return EMPTY_TUNNEL_FORM_VALUES;
	const listenPort = String(definition.listenPort);
	const isTargetPortLinked = definition.targetPort === undefined;
	return {
		name: definition.name,
		host: definition.host,
		listenPort,
		targetPort: isTargetPortLinked ? listenPort : String(definition.targetPort),
		isTargetPortLinked,
		targetHost: definition.targetHost ?? '',
		listenAddress: definition.listenAddress ?? '',
		tags: definition.tags.join(TAGS_SEPARATOR),
	};
}
