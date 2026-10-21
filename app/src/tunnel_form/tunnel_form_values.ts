import type {
	OpenHostDto,
	OpenSchemeDto,
	TunnelKindDto,
	TunnelSpecDto,
} from '#/daemon/protocol';

export const DEFAULT_OPEN_SCHEME: OpenSchemeDto = 'http';

/** An empty host inherits the general `openHost` setting. */
export type OpenHostChoice = OpenHostDto | '';

export type TunnelFormValues = {
	kind: TunnelKindDto;
	host: string;
	listenPort: string;
	targetPort: string;
	isTargetPortLinked: boolean;
	targetHost: string;
	listenAddress: string;
	tags: string;
	openScheme: OpenSchemeDto;
	openPath: string;
	openHost: OpenHostChoice;
};

export const EMPTY_TUNNEL_FORM_VALUES: TunnelFormValues = {
	kind: 'local',
	host: '',
	listenPort: '',
	targetPort: '',
	isTargetPortLinked: true,
	targetHost: '',
	listenAddress: '',
	tags: '',
	openScheme: DEFAULT_OPEN_SCHEME,
	openPath: '',
	openHost: '',
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
		kind: definition.kind,
		host: definition.host,
		listenPort,
		targetPort: isTargetPortLinked ? listenPort : String(definition.targetPort),
		isTargetPortLinked,
		targetHost: definition.targetHost ?? '',
		listenAddress: definition.listenAddress ?? '',
		tags: definition.tags.join(TAGS_SEPARATOR),
		openScheme: definition.open?.scheme ?? DEFAULT_OPEN_SCHEME,
		openPath: definition.open?.path ?? '',
		openHost: definition.open?.host ?? '',
	};
}
