import type { TemporaryTunnelSpecDto, TunnelSpecDto } from '#/daemon/protocol';
import {
	isWebPort,
	type DevslotPortEntry,
} from '#/import/devslot/devslot_ports';
import {
	DEVSLOT_LISTEN_ADDRESS,
	getLocalPortText,
	type DevslotLocalPorts,
} from '#/import/devslot/find_devslot_problems';
import { parsePort } from '#/tunnel_form/parse_port';

export const DEVSLOT_TAG = 'devslot';

export type DevslotStartItem<TSpec> = {
	key: string;
	spec: TSpec;
};

type BuildDevslotSpecsInput = {
	entries: readonly DevslotPortEntry[];
	selectedKeys: ReadonlySet<string>;
	localPorts: DevslotLocalPorts;
	host: string;
};

type BaseSpec = Omit<TunnelSpecDto, 'tags' | 'standby'>;

function buildBaseSpec(
	entry: Readonly<DevslotPortEntry>,
	listenPort: number,
	host: string
): BaseSpec {
	const { port } = entry;
	const hostName = isWebPort(port) ? port.hostName : undefined;
	return {
		host,
		kind: 'local',
		listenPort,
		listenAddress: DEVSLOT_LISTEN_ADDRESS,
		...(port.port !== listenPort && { targetPort: port.port }),
		...(hostName !== undefined && { open: { hostName } }),
	};
}

function selectBaseSpecs(
	input: Readonly<BuildDevslotSpecsInput>
): { entry: DevslotPortEntry; spec: BaseSpec }[] {
	return input.entries.flatMap((entry) => {
		if (!input.selectedKeys.has(entry.key)) return [];
		const listenPort = parsePort(getLocalPortText(entry, input.localPorts));
		if (listenPort === null) return [];
		return [{ entry, spec: buildBaseSpec(entry, listenPort, input.host) }];
	});
}

/** Temporary tunnels carry no tag: the devslot origin lets a later discovery recognise them. */
export function buildTemporaryDevslotSpecs(
	input: Readonly<BuildDevslotSpecsInput>
): DevslotStartItem<TemporaryTunnelSpecDto>[] {
	return selectBaseSpecs(input).map(({ entry, spec }) => ({
		key: entry.key,
		spec: {
			...spec,
			devslot: {
				...(entry.instance.projectTag !== undefined && {
					projectTag: entry.instance.projectTag,
				}),
			},
		},
	}));
}

export function buildSavedDevslotSpecs(
	input: Readonly<BuildDevslotSpecsInput>
): DevslotStartItem<TunnelSpecDto>[] {
	return selectBaseSpecs(input).map(({ entry, spec }) => ({
		key: entry.key,
		spec: {
			...spec,
			tags: [
				DEVSLOT_TAG,
				...(entry.instance.projectTag === undefined
					? []
					: [entry.instance.projectTag]),
			],
		},
	}));
}
