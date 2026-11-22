import { useLingui } from '@lingui/react/macro';
import { Button, Switch } from '@minimalstuff/ui';
import type { ChangeEvent } from 'react';

import type { DevslotDiscoveryDto, SettingsDto } from '#/daemon/protocol';
import { DevslotInstanceFieldset } from '#/import/devslot/devslot_instance_fieldset';
import { describeStartAction } from '#/import/devslot/devslot_texts';
import { useDevslotForm } from '#/import/devslot/use_devslot_form';
import { FormErrorSummary } from '#/shared/form_error_summary';

type DevslotStartFormProps = {
	discovery: DevslotDiscoveryDto;
	settings: SettingsDto;
	removedIds: ReadonlySet<string>;
};

export function DevslotStartForm({
	discovery,
	settings,
	removedIds,
}: Readonly<DevslotStartFormProps>) {
	const { t, i18n } = useLingui();
	const form = useDevslotForm(discovery, settings, removedIds);
	const selectedCount = form.selectedKeys.size;

	const handlePersistenceChange = (event: ChangeEvent<HTMLInputElement>) =>
		form.changePersistence(event.target.checked);

	return (
		<form noValidate onSubmit={form.submit} className="flex flex-col gap-4">
			<FormErrorSummary
				messages={form.summary.messages}
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
				<div>
					<Button
						type="submit"
						startIcon="i-mdi-play"
						loading={form.isStarting}
						disabled={selectedCount === 0}
					>
						{describeStartAction(selectedCount, form.isPersistent, i18n)}
					</Button>
				</div>
			</div>
		</form>
	);
}
