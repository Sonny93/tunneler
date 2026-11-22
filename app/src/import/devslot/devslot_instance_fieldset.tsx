import { useLingui } from '@lingui/react/macro';
import { useId } from 'react';

import { DevslotPortRow } from '#/import/devslot/devslot_port_row';
import type { DevslotPortEntry } from '#/import/devslot/devslot_ports';
import {
	getLocalPortText,
	type DevslotLocalPorts,
	type DevslotPortProblem,
} from '#/import/devslot/find_devslot_problems';

type DevslotInstanceFieldsetProps = {
	entries: readonly DevslotPortEntry[];
	selectedKeys: ReadonlySet<string>;
	problems: ReadonlyMap<string, DevslotPortProblem | null>;
	localPorts: DevslotLocalPorts;
	isShowingErrors: boolean;
	onToggle: (key: string) => void;
	onLocalPortChange: (key: string, text: string) => void;
};

/** One instance: its identity as text (never links) and its ports. All entries belong to the same instance. */
export function DevslotInstanceFieldset({
	entries,
	selectedKeys,
	problems,
	localPorts,
	isShowingErrors,
	onToggle,
	onLocalPortChange,
}: Readonly<DevslotInstanceFieldsetProps>) {
	const { t } = useLingui();
	const detailsId = useId();
	const instance = entries[0]?.instance;
	if (instance === undefined) return null;
	const { appUrl, project, path } = instance;

	return (
		<fieldset
			aria-describedby={detailsId}
			className="m-0 flex flex-col gap-3 border-0 p-0"
		>
			<legend className="mb-1 text-sm font-semibold">{instance.name}</legend>
			<div id={detailsId} className="flex flex-col text-sm text-tn-muted">
				{project !== undefined && <span>{t`Project: ${project}`}</span>}
				<span>{t`Path: ${path}`}</span>
				{appUrl !== undefined && <span>{t`App: ${appUrl}`}</span>}
				{instance.servicesRunning === false && (
					<span>{t`Services are not running`}</span>
				)}
			</div>
			{entries.map((entry) => (
				<DevslotPortRow
					key={entry.key}
					entry={entry}
					isSelected={selectedKeys.has(entry.key)}
					problem={problems.get(entry.key) ?? null}
					localPortText={getLocalPortText(entry, localPorts)}
					isShowingErrors={isShowingErrors}
					onToggle={onToggle}
					onLocalPortChange={onLocalPortChange}
				/>
			))}
		</fieldset>
	);
}
