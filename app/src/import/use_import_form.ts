import { useState, type FormEvent } from 'react';

import { useUiStore } from '#/state/ui_store';
import { parseTags } from '#/tunnel_form/parse_tags';
import { useDaemonStore } from '#/state/daemon_store';
import { useToastStore } from '#/shared/toasts/toast_store';
import { describeImportedCount } from '#/import/import_texts';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { buildImportSpecs } from '#/import/build_import_specs';
import { useImportSelection } from '#/import/use_import_selection';
import { proposeImportNames } from '#/import/propose_import_names';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import type { ImportPreviewDto, SettingsDto } from '#/daemon/protocol';
import {
	findImportConflicts,
	type ImportConflict,
} from '#/import/find_import_conflicts';
import {
	createImportedTunnels,
	type ImportFailure,
	type ImportOutcome,
} from '#/import/create_imported_tunnels';

const MISSING_HOST_ERROR = "Choisissez l'hôte des tunnels sans destination.";
const HOST_FIELD_NAME = 'defaultHost';

export type ImportProgress = { doneCount: number; totalCount: number };

type UseImportFormReturn = {
	names: readonly string[];
	conflicts: readonly (ImportConflict | null)[];
	selection: ReturnType<typeof useImportSelection>;
	defaultHost: string;
	hostError: string | undefined;
	tagsText: string;
	progress: ImportProgress | null;
	summary: ReturnType<typeof useFormErrorSummary>;
	summaryTitle: string;
	changeDefaultHost: (host: string) => void;
	changeTags: (text: string) => void;
	submit: (event: FormEvent<HTMLFormElement>) => void;
};

function describeFailure({ name, messages }: Readonly<ImportFailure>): string {
	return `${name} : ${messages.join(', ')}`;
}

function describeSummaryTitle(
	createdCount: number,
	failedCount: number
): string {
	if (createdCount === 0) return 'Import impossible :';
	return `${describeImportedCount(createdCount)}, ${failedCount} en échec :`;
}

export function useImportForm(
	preview: Readonly<ImportPreviewDto>,
	settings: Readonly<SettingsDto>
): UseImportFormReturn {
	const gateway = useDaemonGateway();
	const tunnels = useDaemonStore((state) => state.tunnels);
	const { candidates } = preview;
	const [names] = useState(() =>
		proposeImportNames(
			candidates,
			tunnels.map((tunnel) => tunnel.name)
		)
	);
	const [initialIndexes] = useState(
		() =>
			new Set(
				findImportConflicts(candidates, tunnels, settings, null).flatMap(
					(conflict, index) => (conflict === null ? [index] : [])
				)
			)
	);
	const selection = useImportSelection(initialIndexes, candidates.length);
	const [defaultHost, setDefaultHost] = useState('');
	const [tagsText, setTagsText] = useState('');
	const [hostError, setHostError] = useState<string>();
	const [progress, setProgress] = useState<ImportProgress | null>(null);
	const [summaryTitle, setSummaryTitle] = useState('');
	const summary = useFormErrorSummary();
	const conflicts = findImportConflicts(
		candidates,
		tunnels,
		settings,
		defaultHost || null
	);

	const changeDefaultHost = (host: string) => {
		setDefaultHost(host);
		setHostError(undefined);
	};

	const reportOutcome = (outcome: Readonly<ImportOutcome>) => {
		selection.deselect(outcome.createdIndexes);
		setSummaryTitle(
			describeSummaryTitle(
				outcome.createdIndexes.length,
				outcome.failures.length
			)
		);
		summary.reportMessages(outcome.failures.map(describeFailure));
	};

	const importSelected = async () => {
		const importSpecs = buildImportSpecs({
			candidates,
			names,
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
			(doneCount) => setProgress({ doneCount, totalCount })
		);
		setProgress(null);
		if (outcome.failures.length > 0) {
			reportOutcome(outcome);
			return;
		}
		useToastStore.getState().pushToast({
			tone: 'success',
			message: describeImportedCount(outcome.createdIndexes.length),
		});
		useUiStore.getState().closePanel();
	};

	const submit = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (progress !== null || selection.selectedIndexes.size === 0) return;
		const isHostMissing =
			defaultHost === '' &&
			[...selection.selectedIndexes].some(
				(index) => candidates[index]?.host === undefined
			);
		if (isHostMissing) {
			setHostError(MISSING_HOST_ERROR);
			const field = event.currentTarget.elements.namedItem(HOST_FIELD_NAME);
			if (field instanceof HTMLElement) field.focus();
			return;
		}
		void importSelected();
	};

	return {
		names,
		conflicts,
		selection,
		defaultHost,
		hostError,
		tagsText,
		progress,
		summary,
		summaryTitle,
		changeDefaultHost,
		changeTags: setTagsText,
		submit,
	};
}
