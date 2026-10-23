import { useLingui } from '@lingui/react/macro';
import { useEffect, useState } from 'react';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import { getSubmissionErrorMessages } from '#/shared/submission_error_messages';

type UseFormErrorSummaryReturn = {
	messages: readonly string[];
	/** Callback ref for the `FormErrorSummary`. */
	attachSummary: (element: HTMLDivElement | null) => void;
	reportFailure: (rejection: unknown) => void;
	reportMessages: (messages: readonly string[]) => void;
	clearFailure: () => void;
};

/** Holds the failure messages of a submission and moves the focus to the summary when they change. */
export function useFormErrorSummary(): UseFormErrorSummaryReturn {
	const { i18n } = useLingui();
	const [messages, setMessages] = useState<readonly string[]>([]);
	const [summary, attachSummary] = useState<HTMLDivElement | null>(null);

	useEffect(() => {
		if (messages.length > 0) summary?.focus();
	}, [messages, summary]);

	const reportFailure = (rejection: unknown) =>
		setMessages(
			getSubmissionErrorMessages(toDaemonCommandError(rejection), i18n)
		);
	const reportMessages = (failureMessages: readonly string[]) =>
		setMessages(failureMessages);
	const clearFailure = () => setMessages([]);

	return {
		messages,
		attachSummary,
		reportFailure,
		reportMessages,
		clearFailure,
	};
}
