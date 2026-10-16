import { getPromptId } from '#/state/prompt_id';
import type { EventMessage, TunnelDto } from '#/daemon/protocol';
import type { DaemonState, PendingPrompt } from '#/state/daemon_state';

type EventOf<TName extends EventMessage['event']> = Extract<
	EventMessage,
	{ event: TName }
>['data'];

function patchTunnel(
	state: DaemonState,
	id: string,
	patch: Readonly<Partial<Pick<TunnelDto, 'state' | 'health' | 'metrics'>>>
): DaemonState {
	if (!state.tunnels.some((tunnel) => tunnel.id === id)) return state;
	return {
		...state,
		tunnels: state.tunnels.map((tunnel) =>
			tunnel.id === id ? { ...tunnel, ...patch } : tunnel
		),
	};
}

function appendPrompt(
	state: DaemonState,
	prompt: Readonly<PendingPrompt>
): DaemonState {
	const promptId = getPromptId(prompt);
	if (state.prompts.some((existing) => getPromptId(existing) === promptId)) {
		return state;
	}
	return { ...state, prompts: [...state.prompts, prompt] };
}

function reduceCredentialRequest(
	state: DaemonState,
	{ promptId, ...request }: EventOf<'credential.request'>
): DaemonState {
	return appendPrompt(state, { kind: 'credential', promptId, request });
}

function reduceHostKeyRequest(
	state: DaemonState,
	data: EventOf<'hostkey.request'>
): DaemonState {
	if (data.kind === 'prompt') {
		const { kind: _kind, ...fields } = data;
		return appendPrompt(state, { kind: 'hostKey', ...fields });
	}
	const alertId = `alert:${data.alias}:${data.presentedFingerprint}`;
	return appendPrompt(state, {
		kind: 'hostKeyAlert',
		alertId,
		alias: data.alias,
		reason: data.reason,
		presentedFingerprint: data.presentedFingerprint,
	});
}

export function reduceDaemonEvent(
	state: DaemonState,
	event: Readonly<EventMessage>
): DaemonState {
	switch (event.event) {
		case 'tunnel.changed':
			return patchTunnel(state, event.data.id, {
				state: event.data.state,
				health: event.data.health,
			});
		case 'tunnel.metrics':
			return patchTunnel(state, event.data.id, { metrics: event.data.metrics });
		case 'config.invalid':
			return { ...state, configErrors: event.data.messages };
		case 'credential.request':
			return reduceCredentialRequest(state, event.data);
		case 'hostkey.request':
			return reduceHostKeyRequest(state, event.data);
		case 'prompt.resolved':
			return {
				...state,
				prompts: state.prompts.filter(
					(prompt) => getPromptId(prompt) !== event.data.promptId
				),
			};
		case 'quit.requested':
			return { ...state, isQuitRequested: true };
		case 'tunnel.log':
		case 'config.reloaded':
			return state;
	}
}
