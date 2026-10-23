import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';
import { useId, type MouseEvent } from 'react';

import { useDaemonAction } from '#/daemon/use_daemon_action';
import { UpdateProgress } from '#/updates/update_progress';
import { useUpdateStore } from '#/updates/update_store';
import { useInstallUpdate } from '#/updates/use_install_update';

export function UpdateNotice() {
	const { t } = useLingui();
	const offer = useUpdateStore((state) => state.offer);
	const isHidden = useUpdateStore((state) => state.isHidden);
	const isInstalling = useUpdateStore((state) => state.isInstalling);
	const hide = useUpdateStore((state) => state.hide);
	const installUpdate = useInstallUpdate();
	const runDaemonAction = useDaemonAction();
	const headingId = useId();

	if (offer === null || isHidden) return null;
	const { version } = offer;

	const handleHide = (event: MouseEvent<HTMLButtonElement>) => {
		event.currentTarget.closest('main')?.focus();
		hide();
	};

	const handleOpenRelease = (event: MouseEvent<HTMLAnchorElement>) => {
		event.preventDefault();
		void runDaemonAction(
			(gateway) => gateway.openReleasePage(),
			t`Couldn't open the page`
		);
	};

	return (
		<section
			aria-labelledby={headingId}
			className="mb-4 flex items-start gap-3 rounded-lg border border-tn-divider bg-tn-info-subtle p-3 text-tn-text"
		>
			<span
				aria-hidden="true"
				className="i-mdi-update mt-0.5 size-5 shrink-0 text-tn-info"
			/>
			<div className="flex min-w-0 flex-1 flex-col items-start gap-2">
				<h2 id={headingId} className="m-0 text-base font-semibold">
					{t`Version ${version} available`}
				</h2>
				{offer.notes !== undefined && (
					<details className="text-sm">
						<summary className="cursor-pointer">{t`Release notes`}</summary>
						<p className="m-0 mt-2 whitespace-pre-line break-words">
							{offer.notes}
						</p>
					</details>
				)}
				{isInstalling ? (
					<UpdateProgress />
				) : (
					<div className="flex flex-wrap items-center gap-2">
						{offer.action === 'installAndRestart' && (
							<Button size="sm" onClick={() => void installUpdate()}>
								{t`Install and restart`}
							</Button>
						)}
						<a
							href={offer.releaseUrl}
							onClick={handleOpenRelease}
							className="inline-flex min-h-6 items-center text-sm underline"
						>
							{t`View release`}
						</a>
						<Button
							size="sm"
							variant="outline"
							color="neutral"
							onClick={handleHide}
						>
							{t`Hide`}
						</Button>
					</div>
				)}
			</div>
		</section>
	);
}
