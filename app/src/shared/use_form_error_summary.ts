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
	/** Shows the messages without moving the focus, for a caller that focuses something else. */
	reportMessagesKeepingFocus: (messages: readonly string[]) => void;
	clearFailure: () => void;
};

type SummaryReport = {
	messages: readonly string[];
	isFocusing: boolean;
};

const EMPTY_REPORT: SummaryReport = { messages: [], isFocusing: false };

/** Holds the failure messages of a submission and moves the focus to the summary when they change. */
export function useFormErrorSummary(): UseFormErrorSummaryReturn {
	const { i18n } = useLingui();
	const [report, setReport] = useState<SummaryReport>(EMPTY_REPORT);
	const [summary, attachSummary] = useState<HTMLDivElement | null>(null);
	const { messages, isFocusing } = report;

	useEffect(() => {
		if (messages.length > 0 && isFocusing) summary?.focus();
	}, [messages, isFocusing, summary]);

	const reportFailure = (rejection: unknown) =>
		setReport({
			messages: getSubmissionErrorMessages(
				toDaemonCommandError(rejection),
				i18n
			),
			isFocusing: true,
		});
	const reportMessages = (failureMessages: readonly string[]) =>
		setReport({ messages: failureMessages, isFocusing: true });
	const reportMessagesKeepingFocus = (failureMessages: readonly string[]) =>
		setReport({ messages: failureMessages, isFocusing: false });
	const clearFailure = () => setReport(EMPTY_REPORT);

	return {
		messages,
		attachSummary,
		reportFailure,
		reportMessages,
		reportMessagesKeepingFocus,
		clearFailure,
	};
}
