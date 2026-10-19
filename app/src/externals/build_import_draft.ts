import type {
	ExternalForwardDto,
	ExternalProcessDto,
	TunnelSpecDto,
} from '#/daemon/protocol';

/** The tunnel spec that would reproduce a forward of an external ssh process. */
export function buildImportDraft(
	process: Readonly<ExternalProcessDto>,
	forward: Readonly<ExternalForwardDto>
): TunnelSpecDto {
	const isTargetPortLinked =
		forward.targetPort === undefined ||
		forward.targetPort === forward.listenPort;
	return {
		host: process.destination ?? '',
		kind: forward.kind,
		listenPort: forward.listenPort,
		listenAddress: forward.listenAddress,
		targetHost: forward.targetHost,
		targetPort: isTargetPortLinked ? undefined : forward.targetPort,
		tags: [],
	};
}
