import type { PendingPrompt } from '#/state/daemon_state';

export function getPromptId(prompt: Readonly<PendingPrompt>): string {
	return prompt.kind === 'hostKeyAlert' ? prompt.alertId : prompt.promptId;
}
