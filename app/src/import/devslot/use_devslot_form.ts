import { msg } from '@lingui/core/macro';
import { useLingui } from '@lingui/react/macro';
import { useEffect, useMemo, useRef, useState, type SubmitEvent } from 'react';

import type { DevslotDiscoveryDto, SettingsDto } from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import {
	buildSavedDevslotSpecs,
	buildTemporaryDevslotSpecs,
} from '#/import/devslot/build_devslot_specs';
import {
	flattenDevslotPorts,
	isPortTunnelable,
} from '#/import/devslot/devslot_ports';
import {
	describeStartedCount,
	describeStarting,
	describeStartOutcome,
} from '#/import/devslot/devslot_texts';
import {
	findDevslotProblems,
	type DevslotLocalPorts,
} from '#/import/devslot/find_devslot_problems';
import { getInitialDevslotSelection } from '#/import/devslot/get_initial_devslot_selection';
import { releaseRemovedPorts } from '#/import/devslot/release_removed_ports';
import {
	createAndStartDevslotTunnels,
	describeDevslotStartFailure,
	startTemporaryDevslotTunnels,
	type DevslotStartOutcome,
} from '#/import/devslot/run_devslot_start';
import { pushToast } from '#/shared/toasts/push_toast';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import { useDaemonStore } from '#/state/daemon_store';
import { useUiStore } from '#/state/ui_store';

const LOCAL_PORT_FIELD_PREFIX = 'localPort-';

type UseDevslotFormReturn = {
	entries: ReturnType<typeof flattenDevslotPorts>;
	selectedKeys: ReadonlySet<string>;
	problems: ReturnType<typeof findDevslotProblems>;
	localPorts: DevslotLocalPorts;
	isPersistent: boolean;
	isShowingErrors: boolean;
	pinnedKeys: ReadonlySet<string>;
	isStarting: boolean;
	statusMessage: string;
	summary: ReturnType<typeof useFormErrorSummary>;
	summaryTitle: string;
	togglePort: (key: string) => void;
	changeLocalPort: (key: string, text: string) => void;
	changePersistence: (isPersistent: boolean) => void;
	submit: (event: SubmitEvent<HTMLFormElement>) => void;
};

export function useDevslotForm(
	discovery: Readonly<DevslotDiscoveryDto>,
	settings: Readonly<SettingsDto>,
	removedIds: ReadonlySet<string>
): UseDevslotFormReturn {
	const { i18n } = useLingui();
	const gateway = useDaemonGateway();
	const storedTunnels = useDaemonStore((state) => state.tunnels);
	const { host } = discovery;
	const tunnels = useMemo(
		() => storedTunnels.filter(({ id }) => !removedIds.has(id)),
		[storedTunnels, removedIds]
	);
	const entries = useMemo(
		() =>
			flattenDevslotPorts(releaseRemovedPorts(discovery.instances, removedIds)),
		[discovery.instances, removedIds]
	);
	const previousEntries = useRef(entries);
	const [selectedKeys, setSelectedKeys] = useState(() =>
		getInitialDevslotSelection(entries, tunnels, settings, host)
	);
	const [localPorts, setLocalPorts] = useState<DevslotLocalPorts>({});
	const [isPersistent, setIsPersistent] = useState(discovery.persistDefault);
	const [isShowingErrors, setIsShowingErrors] = useState(false);
	const [pinnedKeys, setPinnedKeys] = useState<ReadonlySet<string>>(new Set());
	const [isStarting, setIsStarting] = useState(false);
	const [statusMessage, setStatusMessage] = useState('');
	const [summaryTitle, setSummaryTitle] = useState('');
	const summary = useFormErrorSummary();
	const problems = findDevslotProblems({
		entries,
		localPorts,
		selectedKeys,
		tunnels,
		settings,
		host,
	});

	useEffect(() => {
		const before = previousEntries.current;
		previousEntries.current = entries;
		const freed = entries.filter(
			(entry) =>
				isPortTunnelable(entry) &&
				before.some(
					({ key, port }) =>
						key === entry.key && port.status === 'alreadyTunneled'
				)
		);
		if (freed.length === 0) return;
		const added = getInitialDevslotSelection(freed, tunnels, settings, host);
		setSelectedKeys((current) => new Set([...current, ...added]));
	}, [entries, tunnels, settings, host]);

	const togglePort = (key: string) =>
		setSelectedKeys((current) => {
			const next = new Set(current);
			if (!next.delete(key)) next.add(key);
			return next;
		});
	const changeLocalPort = (key: string, text: string) =>
		setLocalPorts((current) => ({ ...current, [key]: text }));

	const reportFailures = (outcome: Readonly<DevslotStartOutcome>) => {
		setSelectedKeys(
			(current) =>
				new Set([...current].filter((key) => !outcome.doneKeys.includes(key)))
		);
		setPinnedKeys(
			(current) =>
				new Set([
					...current,
					...outcome.failures
						.filter(({ stage }) => stage !== 'start')
						.map(({ key }) => key),
				])
		);
		const startedCount = outcome.startedCount;
		const failedCount = outcome.failures.length;
		setSummaryTitle(
			i18n._(msg`${startedCount} started, ${failedCount} failed:`)
		);
		setStatusMessage(describeStartOutcome(startedCount, failedCount, i18n));
		summary.reportMessages(
			outcome.failures.map((failure) =>
				describeDevslotStartFailure(failure, i18n)
			)
		);
	};

	const startSelected = async () => {
		const input = { entries, selectedKeys, localPorts, host };
		setIsStarting(true);
		setStatusMessage(describeStarting(selectedKeys.size, i18n));
		summary.clearFailure();
		const outcome = isPersistent
			? await createAndStartDevslotTunnels(
					gateway,
					buildSavedDevslotSpecs(input),
					i18n
				)
			: await startTemporaryDevslotTunnels(
					gateway,
					buildTemporaryDevslotSpecs(input),
					i18n
				);
		setIsStarting(false);
		if (outcome.failures.length > 0) {
			reportFailures(outcome);
			return;
		}
		pushToast({
			tone: 'success',
			message: describeStartedCount(outcome.startedCount, i18n),
		});
		useUiStore.getState().closePanel();
	};

	const focusField = (form: HTMLFormElement, key: string) => {
		const field = form.elements.namedItem(`${LOCAL_PORT_FIELD_PREFIX}${key}`);
		if (field instanceof HTMLElement) field.focus();
	};

	const submit = (event: SubmitEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (isStarting || selectedKeys.size === 0) return;
		const invalidKey = entries.find(
			({ key }) =>
				selectedKeys.has(key) &&
				problems.get(key) !== null &&
				problems.get(key) !== undefined
		)?.key;
		if (invalidKey !== undefined) {
			setIsShowingErrors(true);
			focusField(event.currentTarget, invalidKey);
			return;
		}
		void startSelected();
	};

	return {
		entries,
		selectedKeys,
		problems,
		localPorts,
		isPersistent,
		isShowingErrors,
		pinnedKeys,
		isStarting,
		statusMessage,
		summary,
		summaryTitle,
		togglePort,
		changeLocalPort,
		changePersistence: setIsPersistent,
		submit,
	};
}
