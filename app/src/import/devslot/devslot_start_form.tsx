import { useLingui } from '@lingui/react/macro';
import { Switch } from '@minimalstuff/ui';
import type { ChangeEvent } from 'react';

import type { DevslotDiscoveryDto } from '#/daemon/protocol';
import { ConflictCheckFailure } from '#/import/conflict_check_failure';
import { DevslotInstanceFieldset } from '#/import/devslot/devslot_instance_fieldset';
import { describeStartAction } from '#/import/devslot/devslot_texts';
import { useDevslotForm } from '#/import/devslot/use_devslot_form';
import { ImportActionButton } from '#/import/import_action_button';
import { FormErrorSummary } from '#/shared/form_error_summary';

type DevslotStartFormProps = {
	discovery: DevslotDiscoveryDto;
	removedIds: ReadonlySet<string>;
};

export function DevslotStartForm({
	discovery,
	removedIds,
}: Readonly<DevslotStartFormProps>) {
	const { t, i18n } = useLingui();
	const form = useDevslotForm(discovery, removedIds);
	const selectedCount = form.selectedKeys.size;

	const handlePersistenceChange = (event: ChangeEvent<HTMLInputElement>) =>
		form.changePersistence(event.target.checked);

	return (
		<form noValidate onSubmit={form.submit} className="flex flex-col gap-4">
			<ConflictCheckFailure hasFailed={form.hasConflictCheckFailed} />
			<FormErrorSummary
				messages={form.summary.messages}
				englishTexts={form.summary.englishTexts}
				title={form.summaryTitle}
				attach={form.summary.attachSummary}
			/>
			{discovery.instances.map((instance, instanceIndex) => (
				<DevslotInstanceFieldset
					key={instance.path}
					entries={form.entries.filter(({ key }) =>
						key.startsWith(`${instanceIndex}:`)
					)}
					selectedKeys={form.selectedKeys}
					problems={form.problems}
					localPorts={form.localPorts}
					isShowingErrors={form.isShowingErrors}
					pinnedKeys={form.pinnedKeys}
					onToggle={form.togglePort}
					onLocalPortChange={form.changeLocalPort}
				/>
			))}
			<div className="sticky bottom-0 flex flex-col gap-2 border-t border-tn-divider bg-tn-surface py-3">
				<Switch
					name="keepTunnels"
					label={t`Keep these tunnels`}
					description={t`On: the tunnels are saved and tagged devslot. Off: they live until stopped.`}
					checked={form.isPersistent}
					onChange={handlePersistenceChange}
				/>
				<output className="block text-sm">{form.statusMessage}</output>
				<ImportActionButton
					startIcon="i-mdi-play"
					isRunning={form.isStarting}
					isChecking={form.isCheckingConflicts}
					isSelectionEmpty={selectedCount === 0}
					emptyHint={t`Select at least one port`}
				>
					{describeStartAction(selectedCount, form.isPersistent, i18n)}
				</ImportActionButton>
			</div>
		</form>
	);
}
