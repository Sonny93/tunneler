import { useLingui } from '@lingui/react/macro';
import { Button, ModalFooter } from '@minimalstuff/ui';
import { useId } from 'react';

import type { SettingsDto } from '#/daemon/protocol';
import type { SettingsTab } from '#/settings/settings_tab';
import { SettingsTabs } from '#/settings/settings_tabs';
import { useConfirmDiscard } from '#/settings/use_confirm_discard';
import { useSettingsForm } from '#/settings/use_settings_form';
import { FormErrorSummary } from '#/shared/form_error_summary';

type SettingsDialogFormProps = {
	settings: SettingsDto;
	initialTab?: SettingsTab;
};

export function SettingsDialogForm({
	settings,
	initialTab,
}: Readonly<SettingsDialogFormProps>) {
	const { t } = useLingui();
	const formId = useId();
	const form = useSettingsForm(settings, initialTab);
	const { attachForm } = form;
	const { confirmDiscard } = useConfirmDiscard(form.isDirty);
	const handleCancel = () => void confirmDiscard();

	return (
		<>
			<form
				ref={attachForm}
				id={formId}
				noValidate
				onSubmit={form.submit}
				className="flex min-h-0 flex-1 flex-col"
			>
				<FormErrorSummary
					messages={form.summary.messages}
					attach={form.summary.attachSummary}
				/>
				<SettingsTabs form={form} />
			</form>
			<ModalFooter>
				<div className="flex justify-end gap-2">
					<Button variant="outline" color="neutral" onClick={handleCancel}>
						{t`Cancel`}
					</Button>
					<Button type="submit" form={formId} loading={form.isSaving}>
						{t`Save`}
					</Button>
				</div>
			</ModalFooter>
		</>
	);
}
