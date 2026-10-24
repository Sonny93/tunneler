import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';

import { describeManualUpdateCheck } from '#/updates/describe_manual_update_check';
import {
	useManualUpdateCheck,
	type ManualUpdateCheckStatus,
} from '#/updates/use_manual_update_check';

function useStatusText(status: Readonly<ManualUpdateCheckStatus>): string {
	const { t, i18n } = useLingui();
	if (status.kind === 'idle') return '';
	if (status.kind === 'checking') return t`Checking for updates…`;
	return describeManualUpdateCheck(status, i18n);
}

export function SettingsUpdateCheck() {
	const { t } = useLingui();
	const { status, check } = useManualUpdateCheck();
	const statusText = useStatusText(status);

	return (
		<div className="flex flex-wrap items-center gap-3">
			<Button
				type="button"
				variant="outline"
				color="neutral"
				loading={status.kind === 'checking'}
				onClick={() => void check()}
			>
				{t`Check now`}
			</Button>
			<output className="text-sm text-tn-text">{statusText}</output>
		</div>
	);
}
