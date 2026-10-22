import { Button } from '@minimalstuff/ui';
import { useId, type MouseEvent } from 'react';
import { useLingui } from '@lingui/react/macro';

import { useUiStore } from '#/state/ui_store';
import { useDaemonStore } from '#/state/daemon_store';
import {
	describeTrayProblem,
	isTrayProblem,
} from '#/tray/tray_status_notice_text';

export function TrayStatusNotice() {
	const { t, i18n } = useLingui();
	const status = useDaemonStore((state) => state.tray);
	const dismissedProblem = useUiStore((state) => state.dismissedTrayProblem);
	const dismissTrayProblem = useUiStore((state) => state.dismissTrayProblem);
	const headingId = useId();

	if (!isTrayProblem(status) || status.kind === dismissedProblem) return null;

	const { heading, body } = describeTrayProblem(status, i18n);

	const handleHide = (event: MouseEvent<HTMLButtonElement>) => {
		event.currentTarget.closest('main')?.focus();
		dismissTrayProblem(status.kind);
	};

	return (
		<section
			aria-labelledby={headingId}
			className="mb-4 flex items-start gap-3 rounded-lg border border-tn-divider bg-tn-info-subtle p-3 text-tn-text"
		>
			<span
				aria-hidden="true"
				className="i-mdi-information-outline mt-0.5 size-5 shrink-0 text-tn-info"
			/>
			<div className="flex min-w-0 flex-1 flex-col items-start gap-2">
				<h2 id={headingId} className="m-0 text-base font-semibold">
					{heading}
				</h2>
				<p className="m-0 break-words text-sm">{body}</p>
				<Button
					size="sm"
					variant="outline"
					color="neutral"
					onClick={handleHide}
				>
					{t`Hide`}
				</Button>
			</div>
		</section>
	);
}
