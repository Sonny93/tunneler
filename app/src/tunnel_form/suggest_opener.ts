import { parsePort } from '#/tunnel_form/parse_port';
import type { AvailableOpenerDto } from '#/daemon/protocol';
import type { TunnelFormValues } from '#/tunnel_form/tunnel_form_values';

/**
 * The opener suggested for the target port of the form: the first available
 * one (the daemon lists the user's before the detected ones) whose ports
 * match. Display only, to word "Automatique (…)": the daemon decides.
 */
export function suggestOpener(
	values: Readonly<
		Pick<TunnelFormValues, 'listenPort' | 'targetPort' | 'isTargetPortLinked'>
	>,
	openers: readonly Readonly<AvailableOpenerDto>[]
): Readonly<AvailableOpenerDto> | null {
	const targetPort = parsePort(
		values.isTargetPortLinked ? values.listenPort : values.targetPort
	);
	if (targetPort === null) return null;
	return openers.find(({ ports }) => ports.includes(targetPort)) ?? null;
}
