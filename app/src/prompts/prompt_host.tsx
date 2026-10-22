import { useLingui } from '@lingui/react/macro';

import { getPromptId } from '#/state/prompt_id';
import { useDaemonStore } from '#/state/daemon_store';
import { useModalCall } from '#/shared/use_modal_call';
import { usePromptActions } from '#/prompts/use_prompt_actions';
import { buildPromptModalRequest } from '#/prompts/prompt_modal_request';

/**
 * Shows the first pending prompt in a lib modal, one at a time (a
 * `<ModalProvider />` must be mounted). A modal the user closes without
 * answering counts as a refusal; a prompt removed from the store (answered
 * elsewhere) closes its modal.
 */
export function PromptHost(): null {
	const firstPrompt = useDaemonStore((state) => state.prompts[0] ?? null);
	const { i18n } = useLingui();
	const actions = usePromptActions();

	const handleDismissed = (dismissedId: string) => {
		const { prompts } = useDaemonStore.getState();
		const prompt = prompts.find(
			(pending) => getPromptId(pending) === dismissedId
		);
		if (!prompt) return;
		if (prompt.kind === 'credential') {
			void actions.answerCredential(dismissedId, null);
			return;
		}
		if (prompt.kind === 'hostKey') {
			void actions.decideHostKey(dismissedId, 'reject');
			return;
		}
		actions.acknowledgeAlert(dismissedId);
	};

	const request =
		firstPrompt && buildPromptModalRequest(firstPrompt, actions, i18n);
	useModalCall(request, handleDismissed);

	return null;
}
