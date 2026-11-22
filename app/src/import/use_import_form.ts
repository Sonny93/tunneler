import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { useState, type SubmitEvent } from 'react';

import type { ImportPreviewDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { buildImportSpecs } from '#/import/build_import_specs';
import {
	createImportedTunnels,
	type ImportFailure,
	type ImportOutcome,
} from '#/import/create_imported_tunnels';
import { describeImportedCount } from '#/import/import_texts';
import {
	useImportConflicts,
	type ImportConflicts,
} from '#/import/use_import_conflicts';
import { useImportSelection } from '#/import/use_import_selection';
import { pushToast } from '#/shared/toasts/push_toast';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import { useUiStore } from '#/state/ui_store';
import { parseTags } from '#/tunnel_form/parse_tags';
import { validateTags } from '#/tunnel_form/validate_tags';

const MISSING_HOST_ERROR = msg`Choose the host of the tunnels without a destination.`;
const HOST_FIELD_NAME = 'defaultHost';
const TAGS_FIELD_NAME = 'tags';

export type ImportProgress = { doneCount: number; totalCount: number };

type UseImportFormReturn = {
	conflicts: ImportConflicts;
	/** `false` until the daemon told which candidates conflict: importing waits for it. */
	isClassified: boolean;
	/** `true` while the last check for conflicts failed. */
	hasConflictCheckFailed: boolean;
	selection: ReturnType<typeof useImportSelection>;
	defaultHost: string;
	hostError: string | undefined;
	tagsText: string;
	tagsError: string | undefined;
	progress: ImportProgress | null;
	summary: ReturnType<typeof useFormErrorSummary>;
	summaryTitle: string;
	changeDefaultHost: (host: string) => void;
	changeTags: (text: string) => void;
	submit: (event: SubmitEvent<HTMLFormElement>) => void;
};

function describeFailure(
	{ label, messages }: Readonly<ImportFailure>,
	i18n: I18n
): string {
	const messageList = messages.join(', ');
	return i18n._(msg`${label}: ${messageList}`);
}

function describeSummaryTitle(
	createdCount: number,
	failedCount: number,
	i18n: I18n
): string {
	if (createdCount === 0) return i18n._(msg`Import failed:`);
	const importedText = describeImportedCount(createdCount, i18n);
	return i18n._(msg`${importedText}, ${failedCount} failed:`);
}

export function useImportForm(
	preview: Readonly<ImportPreviewDto>
): UseImportFormReturn {
	const { i18n } = useLingui();
	const gateway = useDaemonGateway();
	const { candidates } = preview;
	const [initialIndexes] = useState(
		() => new Set(candidates.map((_, index) => index))
	);
	const selection = useImportSelection(initialIndexes, candidates.length);
	const [defaultHost, setDefaultHost] = useState('');
	const deselectConflicting = (firstConflicts: ImportConflicts) =>
		selection.deselect(
			firstConflicts.flatMap((conflict, index) =>
				conflict === null ? [] : [index]
			)
		);
	const { conflicts, isClassified, hasFailed } = useImportConflicts(
		candidates,
		defaultHost,
		deselectConflicting
	);
	const [tagsText, setTagsText] = useState('');
	const [hostError, setHostError] = useState<string>();
	const [tagsError, setTagsError] = useState<string>();
	const [progress, setProgress] = useState<ImportProgress | null>(null);
	const [summaryTitle, setSummaryTitle] = useState('');
	const summary = useFormErrorSummary();

	const changeDefaultHost = (host: string) => {
		setDefaultHost(host);
		setHostError(undefined);
	};

	const changeTags = (text: string) => {
		setTagsText(text);
		setTagsError(undefined);
	};

	const focusField = (form: HTMLFormElement, fieldName: string) => {
		const field = form.elements.namedItem(fieldName);
		if (field instanceof HTMLElement) field.focus();
	};

	const reportOutcome = (outcome: Readonly<ImportOutcome>) => {
		selection.deselect(outcome.createdIndexes);
		setSummaryTitle(
			describeSummaryTitle(
				outcome.createdIndexes.length,
				outcome.failures.length,
				i18n
			)
		);
		summary.reportMessages(
			outcome.failures.map((failure) => describeFailure(failure, i18n)),
			outcome.failures.flatMap((failure) => failure.englishTexts)
		);
	};

	const importSelected = async () => {
		const importSpecs = buildImportSpecs({
			candidates,
			selectedIndexes: selection.selectedIndexes,
			fallbackHost: defaultHost,
			tags: parseTags(tagsText),
		});
		const totalCount = importSpecs.length;
		setProgress({ doneCount: 0, totalCount });
		summary.clearFailure();
		const outcome = await createImportedTunnels(
			gateway,
			importSpecs,
			(doneCount) => setProgress({ doneCount, totalCount }),
			i18n
		);
		setProgress(null);
		if (outcome.failures.length > 0) {
			reportOutcome(outcome);
			return;
		}
		pushToast({
			tone: 'success',
			message: describeImportedCount(outcome.createdIndexes.length, i18n),
		});
		useUiStore.getState().closePanel();
	};

	const submit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (!isClassified || progress !== null) return;
		if (selection.selectedIndexes.size === 0) return;
		const isHostMissing =
			defaultHost === '' &&
			[...selection.selectedIndexes].some(
				(index) => candidates[index]?.host === undefined
			);
		if (isHostMissing) {
			setHostError(i18n._(MISSING_HOST_ERROR));
			focusField(event.currentTarget, HOST_FIELD_NAME);
			return;
		}
		const tagsProblem = validateTags(tagsText);
		const foundTagsError =
			tagsProblem === null ? undefined : i18n._(tagsProblem);
		setTagsError(foundTagsError);
		if (foundTagsError !== undefined) {
			focusField(event.currentTarget, TAGS_FIELD_NAME);
			return;
		}
		void importSelected();
	};

	return {
		conflicts,
		isClassified,
		hasConflictCheckFailed: hasFailed,
		selection,
		defaultHost,
		hostError,
		tagsText,
		tagsError,
		progress,
		summary,
		summaryTitle,
		changeDefaultHost,
		changeTags,
		submit,
	};
}
