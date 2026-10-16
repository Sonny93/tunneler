import { getPromptId } from '#/state/prompt_id';
import type { PendingPrompt } from '#/state/daemon_state';
import type { ModalCallRequest } from '#/shared/use_modal_call';
import type { PromptActions } from '#/prompts/use_prompt_actions';
import { getHostKeyAlertTitle } from '#/prompts/host_key_alert_text';
import { getCredentialTitle } from '#/prompts/credential_prompt_text';
import { HostKeyAlertContent } from '#/prompts/host_key_alert_content';
import { HostKeyPromptContent } from '#/prompts/host_key_prompt_content';
import { CredentialPromptContent } from '#/prompts/credential_prompt_content';

const HOST_KEY_TITLE = 'Hôte inconnu';
const CLOSE_LABEL = 'Fermer';

/** The lib modal showing `prompt`; each answer goes through `actions` and removes the prompt from the store. */
export function buildPromptModalRequest(
	prompt: Readonly<PendingPrompt>,
	actions: Readonly<PromptActions>
): ModalCallRequest {
	const id = getPromptId(prompt);
	switch (prompt.kind) {
		case 'credential':
			return {
				id,
				props: {
					title: getCredentialTitle(prompt.request),
					size: 'sm',
					closeLabel: CLOSE_LABEL,
					children: (
						<CredentialPromptContent
							request={prompt.request}
							onSubmit={(answer) => void actions.answerCredential(id, answer)}
							onCancel={() => void actions.answerCredential(id, null)}
						/>
					),
				},
			};
		case 'hostKey':
			return {
				id,
				props: {
					title: HOST_KEY_TITLE,
					closeLabel: CLOSE_LABEL,
					children: (
						<HostKeyPromptContent
							prompt={prompt}
							onDecide={(decision) => void actions.decideHostKey(id, decision)}
						/>
					),
				},
			};
		case 'hostKeyAlert':
			return {
				id,
				props: {
					title: getHostKeyAlertTitle(prompt.reason),
					dismissible: false,
					children: (
						<HostKeyAlertContent
							alert={prompt}
							onAcknowledge={() => actions.acknowledgeAlert(id)}
						/>
					),
				},
			};
	}
}
