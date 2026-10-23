import { useLingui } from '@lingui/react/macro';
import { useId } from 'react';

export function X11SecurityNote() {
	const { t } = useLingui();
	const titleId = useId();

	return (
		<div
			role="note"
			aria-labelledby={titleId}
			className="flex flex-col gap-1 rounded-md border border-tn-warning bg-tn-warning-subtle p-3 text-sm text-tn-text"
		>
			<p id={titleId} className="m-0 flex items-center gap-1.5 font-semibold">
				<span aria-hidden="true" className="i-mdi-alert size-4 shrink-0" />
				{t`Security`}
			</p>
			<p className="m-0">
				{t`X11 forwarding gives the server full access to your local display: it can capture the keyboard and the screen. Only enable it for trusted hosts.`}
			</p>
		</div>
	);
}
