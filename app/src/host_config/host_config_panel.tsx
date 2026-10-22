import { Button } from '@minimalstuff/ui';
import { Trans, useLingui } from '@lingui/react/macro';

import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import { HostConfigView } from '#/host_config/host_config_view';
import { useResolvedHost } from '#/host_config/use_resolved_host';
import { describeDaemonError } from '#/daemon/describe_daemon_error';

type HostConfigPanelProps = {
	host: string;
};

export function HostConfigPanel({ host }: Readonly<HostConfigPanelProps>) {
	const { t, i18n } = useLingui();
	const closePanel = useUiStore((state) => state.closePanel);
	const { state, retry } = useResolvedHost(host);
	const errorMessage =
		state.status === 'error' ? describeDaemonError(state.error, i18n) : '';

	return (
		<Drawer isOpen title={t`Configuration of ${host}`} onClose={closePanel}>
			<div
				aria-busy={state.status === 'loading'}
				className="flex flex-col gap-4"
			>
				<p className="m-0 text-sm text-tn-muted">
					<Trans>
						Effective configuration, equivalent to <code>ssh -G {host}</code>.
					</Trans>
				</p>
				{state.status === 'loading' && (
					<p className="m-0 text-sm text-tn-muted">
						{t`Reading the configuration…`}
					</p>
				)}
				{state.status === 'error' && (
					<div role="alert" className="flex flex-col items-start gap-2 text-sm">
						<p className="m-0 text-tn-danger">
							{t`Couldn't read the configuration: ${errorMessage}`}
						</p>
						<Button
							startIcon="i-mdi-refresh"
							variant="outline"
							color="neutral"
							onClick={retry}
						>
							{t`Retry`}
						</Button>
					</div>
				)}
				{state.status === 'ready' && (
					<HostConfigView resolvedHost={state.resolvedHost} />
				)}
			</div>
		</Drawer>
	);
}
