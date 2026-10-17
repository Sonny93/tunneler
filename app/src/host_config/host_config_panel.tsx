import { Button } from '@minimalstuff/ui';

import { Drawer } from '#/shared/drawer';
import { useUiStore } from '#/state/ui_store';
import { HostConfigView } from '#/host_config/host_config_view';
import { useResolvedHost } from '#/host_config/use_resolved_host';

type HostConfigPanelProps = {
	host: string;
};

export function HostConfigPanel({ host }: Readonly<HostConfigPanelProps>) {
	const closePanel = useUiStore((state) => state.closePanel);
	const { state, retry } = useResolvedHost(host);

	return (
		<Drawer isOpen title={`Configuration de ${host}`} onClose={closePanel}>
			<div
				aria-busy={state.status === 'loading'}
				className="flex flex-col gap-4"
			>
				<p className="m-0 text-sm text-tn-muted">
					Configuration effective, équivalent de <code>ssh -G {host}</code>.
				</p>
				{state.status === 'loading' && (
					<p className="m-0 text-sm text-tn-muted">
						Lecture de la configuration…
					</p>
				)}
				{state.status === 'error' && (
					<div role="alert" className="flex flex-col items-start gap-2 text-sm">
						<p className="m-0 text-tn-danger">
							Lecture impossible : {state.message}
						</p>
						<Button
							startIcon="i-mdi-refresh"
							variant="outline"
							color="neutral"
							onClick={retry}
						>
							Réessayer
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
