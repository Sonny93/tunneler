import { parsePort } from '#/tunnel_form/parse_port';
import { parseTags } from '#/tunnel_form/parse_tags';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';
import type { TemporaryTunnelSpecDto, TunnelSpecDto } from '#/daemon/protocol';

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
	values: Readonly<TunnelFormValues>
): TunnelSpecDto {
	const hasTarget = values.kind !== 'dynamic';
	return {
		host: values.host.trim(),
		kind: values.kind,
		listenPort: toPort(values.listenPort),
		listenAddress: toOptionalText(values.listenAddress),
		targetHost: hasTarget ? toOptionalText(values.targetHost) : undefined,
		targetPort:
			hasTarget && !values.isTargetPortLinked
				? toPort(values.targetPort)
				: undefined,
		tags: parseTags(values.tags),
	};
}

/** Builds the spec of a temporary tunnel: the same fields, without tags. */
export function toTemporaryTunnelSpec(
	values: Readonly<TunnelFormValues>
): TemporaryTunnelSpecDto {
	const { tags: _tags, ...spec } = toTunnelSpec(values);
	return spec;
}
