import { useMemo } from 'react';
import { i18n } from '@lingui/core';
import { t } from '@lingui/core/macro';

import { useDaemonStore } from '#/state/daemon_store';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import type { CredentialAnswer } from '#/daemon/daemon_gateway';
import { describeDaemonError } from '#/daemon/describe_daemon_error';
import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import type { HostKeyDecisionDto, RememberDto } from '#/daemon/protocol';

const PROMPT_NOT_PENDING_CODE = 'prompt_not_pending';

export type PromptActions = {
	answerCredential: (
		promptId: string,
		answer: CredentialAnswer,
		remember?: RememberDto
	) => Promise<void>;
	decideHostKey: (
		promptId: string,
		decision: HostKeyDecisionDto
	) => Promise<void>;
	acknowledgeAlert: (alertId: string) => void;
};

/** Sends the answer, then removes the prompt whatever the outcome; "no longer pending" is not an error. */
async function respondAndDismiss(
	promptId: string,
	sendResponse: () => Promise<void>
): Promise<void> {
	try {
		await sendResponse();
	} catch (rejection) {
		const error = toDaemonCommandError(rejection);
		if (error.code !== PROMPT_NOT_PENDING_CODE) {
			const failureMessage = t`Couldn't send the answer`;
			const errorMessage = describeDaemonError(error, i18n);
			useToastStore.getState().pushToast({
				tone: 'error',
				message: t`${failureMessage}: ${errorMessage}`,
			});
		}
	} finally {
		useDaemonStore.getState().dismissPrompt(promptId);
	}
}

export function usePromptActions(): PromptActions {
	const gateway = useDaemonGateway();

	return useMemo(
		() => ({
			answerCredential: (promptId, answer, remember) =>
				respondAndDismiss(promptId, () =>
					gateway.respondCredential(promptId, answer, remember)
				),
			decideHostKey: (promptId, decision) =>
				respondAndDismiss(promptId, () =>
					gateway.respondHostKey(promptId, decision)
				),
			acknowledgeAlert: (alertId) =>
				useDaemonStore.getState().dismissPrompt(alertId),
		}),
		[gateway]
	);
}
