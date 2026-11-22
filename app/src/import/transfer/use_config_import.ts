import { useLingui } from '@lingui/react/macro';
import { useRef, useState, type RefObject } from 'react';

import type {
	ConfigImportPlanDto,
	ConfigImportReportDto,
	PickedImportFileDto,
} from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { describePreviewUpdated } from '#/import/import_texts';
import {
	getDefaultSelection,
	mergeRemapHosts,
	withHostAlias,
} from '#/import/transfer/config_import_plan';
import { useImportSelection } from '#/import/use_import_selection';
import { useAnnouncerStore } from '#/shared/announcer/announcer_store';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';

type HostMap = Readonly<Record<string, string>>;
type Busy = 'picking' | 'previewing' | 'applying' | null;

export type LoadedFile = PickedImportFileDto & {
	plan: ConfigImportPlanDto;
	/** What the user chose; it shows at once, before the preview catches up. */
	hostMap: HostMap;
	/** The host map `plan` was made with; it differs from `hostMap` while the preview is out of date. */
	plannedHostMap: HostMap;
	remapHosts: readonly string[];
	/** Changes with every picked file, not with every re-preview. */
	revision: number;
};

export type ConfigImportReport = {
	/** The revision of the file it comes from. */
	revision: number;
	fileName: string;
	report: ConfigImportReportDto;
	/** The entries of the plan that was applied, to name the skipped ones. */
	plan: ConfigImportPlanDto;
};

type UseConfigImportReturn = {
	loaded: LoadedFile | null;
	report: ConfigImportReport | null;
	busy: Busy;
	/** Its changes count as the user's own choice, which a new preview resets. */
	selection: ReturnType<typeof useImportSelection>;
	summary: ReturnType<typeof useFormErrorSummary>;
	chooseFile: () => void;
	changeAlias: (fileHost: string, alias: string) => void;
	apply: () => void;
};

/** `buttonRef` is the button that opens the native dialog: it takes the focus back when the dialog closes. */
export function useConfigImport(
	buttonRef: RefObject<HTMLButtonElement | null>
): UseConfigImportReturn {
	const { i18n } = useLingui();
	const gateway = useDaemonGateway();
	const [loaded, setLoaded] = useState<LoadedFile | null>(null);
	const [report, setReport] = useState<ConfigImportReport | null>(null);
	const [busy, setBusy] = useState<Busy>(null);
	const base = useImportSelection(new Set(), 0);
	const loadedRef = useRef<LoadedFile | null>(null);
	const lastRevision = useRef(0);
	const isRefreshing = useRef(false);
	const isSelectionCustom = useRef(false);
	const summary = useFormErrorSummary();

	const commitLoaded = (file: LoadedFile | null) => {
		loadedRef.current = file;
		setLoaded(file);
	};
	const showPlan = (file: LoadedFile) => {
		commitLoaded(file);
		base.replace(getDefaultSelection(file.plan));
		isSelectionCustom.current = false;
	};
	const customize = <TArguments extends unknown[]>(
		change: (...changeArguments: TArguments) => void
	) => {
		return (...changeArguments: TArguments) => {
			isSelectionCustom.current = true;
			change(...changeArguments);
		};
	};
	const selection = {
		...base,
		toggle: customize(base.toggle),
		selectAll: customize(base.selectAll),
		selectNone: customize(base.selectNone),
		deselect: customize(base.deselect),
		replace: customize(base.replace),
	};

	const loadPicked = async (picked: PickedImportFileDto) => {
		const plan = await gateway.previewConfigImport(picked.contents, {});
		lastRevision.current += 1;
		setReport(null);
		const hostMap: HostMap = {};
		showPlan({
			...picked,
			plan,
			hostMap,
			plannedHostMap: hostMap,
			remapHosts: plan.unknownHosts,
			revision: lastRevision.current,
		});
	};

	const run = async (
		kind: 'picking' | 'applying',
		action: () => Promise<void>
	) => {
		if (busy !== null) return;
		setBusy(kind);
		summary.clearFailure();
		try {
			await action();
		} catch (rejection) {
			if (kind === 'picking') commitLoaded(null);
			summary.reportFailure(rejection);
		} finally {
			setBusy(null);
		}
	};

	const chooseFile = () =>
		void run('picking', async () => {
			const picked = await gateway.pickImportFile();
			if (picked !== null) await loadPicked(picked);
			buttonRef.current?.focus();
		});

	/** Previews the latest choice; an answer to an older choice is dropped and the newer one is asked for. */
	const refreshPreview = async () => {
		if (isRefreshing.current) return;
		isRefreshing.current = true;
		setBusy('previewing');
		summary.clearFailure();
		try {
			for (;;) {
				const file = loadedRef.current;
				if (file === null || file.hostMap === file.plannedHostMap) return;
				const { hostMap } = file;
				// oxlint-disable-next-line no-await-in-loop -- one preview at a time; the loop asks again for the latest choice
				const plan = await gateway.previewConfigImport(file.contents, hostMap);
				const latest = loadedRef.current;
				if (latest === null || latest.hostMap !== hostMap) continue;
				const wasCustom = isSelectionCustom.current;
				showPlan({
					...latest,
					plan,
					plannedHostMap: hostMap,
					remapHosts: mergeRemapHosts(latest.remapHosts, plan),
				});
				useAnnouncerStore
					.getState()
					.announce(
						describePreviewUpdated(
							getDefaultSelection(plan).size,
							wasCustom,
							i18n
						),
						'polite'
					);
			}
		} catch (rejection) {
			summary.reportFailure(rejection);
		} finally {
			isRefreshing.current = false;
			setBusy(null);
		}
	};

	const changeAlias = (fileHost: string, alias: string) => {
		const current = loadedRef.current;
		if (current === null || busy === 'applying' || busy === 'picking') return;
		commitLoaded({
			...current,
			hostMap: withHostAlias(current.hostMap, fileHost, alias),
		});
		void refreshPreview();
	};

	const apply = () =>
		void run('applying', async () => {
			if (loaded === null || loaded.hostMap !== loaded.plannedHostMap) return;
			const indexes = [...selection.selectedIndexes].sort((a, b) => a - b);
			const result = await gateway.applyConfigImport(
				loaded.contents,
				loaded.hostMap,
				indexes
			);
			setReport({
				revision: loaded.revision,
				fileName: loaded.fileName,
				report: result,
				plan: loaded.plan,
			});
			commitLoaded(null);
		});

	return {
		loaded,
		report,
		busy,
		selection,
		summary,
		chooseFile,
		changeAlias,
		apply,
	};
}
