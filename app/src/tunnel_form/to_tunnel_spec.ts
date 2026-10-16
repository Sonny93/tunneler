import { parsePort } from '#/tunnel_form/parse_port';
import { parseTags } from '#/tunnel_form/parse_tags';
import type { TunnelKindDto, TunnelSpecDto } from '#/daemon/protocol';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';

function toOptionalText(text: string): string | undefined {
	const trimmed = text.trim();
	return trimmed === '' ? undefined : trimmed;
}

function toPort(text: string): number {
	const port = parsePort(text);
	if (port === null) throw new RangeError(`Invalid port: "${text}"`);
	return port;
}

/** Builds the spec of valid form values; blank optional texts are omitted. */
export function toTunnelSpec(
	values: Readonly<TunnelFormValues>,
	kind: TunnelKindDto
): TunnelSpecDto {
	return {
		name: values.name.trim(),
		host: values.host.trim(),
		kind,
		listenPort: toPort(values.listenPort),
		listenAddress: toOptionalText(values.listenAddress),
		targetHost: toOptionalText(values.targetHost),
		targetPort: values.isTargetPortLinked
			? undefined
			: toPort(values.targetPort),
		tags: parseTags(values.tags),
	};
}
