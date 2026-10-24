import type { AvailableOpenerDto, OpenDefaultDto } from '#/daemon/protocol';
import { parsePort } from '#/tunnel_form/parse_port';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';

/**
 * The opener suggested for the target port of the form: the first available
 * one (the daemon lists the user's before the detected ones) whose ports
 * match, unless the general setting opens the browser. Display only, to word
 * "Automatique (…)": the daemon decides.
 */
export function suggestOpener(
	values: Readonly<
		Pick<TunnelFormValues, 'listenPort' | 'targetPort' | 'isTargetPortLinked'>
	>,
	openers: readonly Readonly<AvailableOpenerDto>[],
	openDefault: OpenDefaultDto
): Readonly<AvailableOpenerDto> | null {
	if (openDefault === 'browser') return null;
	const targetPort = parsePort(
		values.isTargetPortLinked ? values.listenPort : values.targetPort
	);
	if (targetPort === null) return null;
	return openers.find(({ ports }) => ports.includes(targetPort)) ?? null;
}
