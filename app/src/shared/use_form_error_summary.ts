import { useLingui } from '@lingui/react/macro';
import { useEffect, useState } from 'react';

import { toDaemonCommandError } from '#/daemon/daemon_command_error';
import {
	getEnglishTexts,
	getSubmissionErrorMessages,
} from '#/shared/submission_error_messages';

type UseFormErrorSummaryReturn = {
	messages: readonly string[];
	/** Parts of `messages` that are raw English from the daemon, to mark `lang="en"`. */
	englishTexts: readonly string[];
	/** Callback ref for the `FormErrorSummary`. */
	attachSummary: (element: HTMLDivElement | null) => void;
	reportFailure: (rejection: unknown) => void;
	/** `englishTexts` are the parts of `messages` that are raw English from the daemon. */
	reportMessages: (
		messages: readonly string[],
		englishTexts?: readonly string[]
	) => void;
	/** Shows the messages without moving the focus, for a caller that focuses something else. */
	reportMessagesKeepingFocus: (
		messages: readonly string[],
		englishTexts?: readonly string[]
	) => void;
	clearFailure: () => void;
};

type SummaryReport = {
	messages: readonly string[];
	englishTexts: readonly string[];
	isFocusing: boolean;
};

const EMPTY_REPORT: SummaryReport = {
	messages: [],
	englishTexts: [],
	isFocusing: false,
};

/** Holds the failure messages of a submission and moves the focus to the summary when they change. */
export function useFormErrorSummary(): UseFormErrorSummaryReturn {
	const { i18n } = useLingui();
	const [report, setReport] = useState<SummaryReport>(EMPTY_REPORT);
	const [summary, attachSummary] = useState<HTMLDivElement | null>(null);
	const { messages, englishTexts, isFocusing } = report;

	useEffect(() => {
		if (messages.length > 0 && isFocusing) summary?.focus();
	}, [messages, isFocusing, summary]);

	const reportFailure = (rejection: unknown) => {
		const error = toDaemonCommandError(rejection);
		setReport({
			messages: getSubmissionErrorMessages(error, i18n),
			englishTexts: getEnglishTexts(error, i18n),
			isFocusing: true,
		});
	};
	const reportMessages = (
		failureMessages: readonly string[],
		failureEnglishTexts: readonly string[] = []
	) =>
		setReport({
			messages: failureMessages,
			englishTexts: failureEnglishTexts,
			isFocusing: true,
		});
	const reportMessagesKeepingFocus = (
		failureMessages: readonly string[],
		failureEnglishTexts: readonly string[] = []
	) =>
		setReport({
			messages: failureMessages,
			englishTexts: failureEnglishTexts,
			isFocusing: false,
		});
	const clearFailure = () => setReport(EMPTY_REPORT);

	return {
		messages,
		englishTexts,
		attachSummary,
		reportFailure,
		reportMessages,
		reportMessagesKeepingFocus,
		clearFailure,
	};
}
