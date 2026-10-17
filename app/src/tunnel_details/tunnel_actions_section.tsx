import { useId } from 'react';
import { Button } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import type { TunnelDto } from '#/daemon/protocol';
import { copyTextWithToast } from '#/shared/copy_text';
import { useTunnelActions } from '#/tunnels/list/use_tunnel_actions';
import { buildSshCommand } from '#/tunnel_details/build_ssh_command';
import { useOpenInBrowser } from '#/tunnel_details/use_open_in_browser';

type TunnelActionsSectionProps = {
	tunnel: TunnelDto;
};

export function TunnelActionsSection({
	tunnel,
}: Readonly<TunnelActionsSectionProps>) {
	const headingId = useId();
	const commandId = useId();
	const browserReasonId = useId();
	const restartReasonId = useId();
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);
	const { restart } = useTunnelActions(tunnel);
	const { target, isRunning, open } = useOpenInBrowser(tunnel);
	const command = buildSshCommand(tunnel);

	const handleCopy = () => void copyTextWithToast(command, 'Commande copiée');
	const handleEdit = () => openTunnelEditor(tunnel.id);

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-3">
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				Actions
			</h3>
			{target !== null && (
				<div className="flex flex-col gap-1">
					<div>
						<Button
							startIcon="i-mdi-open-in-new"
							variant="outline"
							color="neutral"
							disabled={!isRunning}
							aria-describedby={isRunning ? undefined : browserReasonId}
							onClick={open}
						>
							Ouvrir dans le navigateur
						</Button>
					</div>
					{!isRunning && (
						<p id={browserReasonId} className="m-0 text-xs text-tn-muted">
							Disponible quand le tunnel est démarré.
						</p>
					)}
				</div>
			)}
			<div className="flex flex-col gap-1">
				<div>
					<Button
						startIcon="i-mdi-content-copy"
						variant="outline"
						color="neutral"
						aria-describedby={commandId}
						onClick={handleCopy}
					>
						Copier la commande ssh
					</Button>
				</div>
				<code
					id={commandId}
					className="block break-all rounded-md bg-tn-surface-raised p-2 font-mono text-xs select-all"
				>
					{command}
				</code>
			</div>
			<div className="flex flex-col gap-1">
				<div className="flex flex-wrap gap-2">
					<Button
						startIcon="i-mdi-restart"
						variant="outline"
						color="neutral"
						disabled={!isRunning}
						aria-describedby={isRunning ? undefined : restartReasonId}
						onClick={restart}
					>
						Redémarrer
					</Button>
					<Button
						startIcon="i-mdi-pencil-outline"
						variant="outline"
						color="neutral"
						onClick={handleEdit}
					>
						Modifier
					</Button>
				</div>
				{!isRunning && (
					<p id={restartReasonId} className="m-0 text-xs text-tn-muted">
						Redémarrage possible quand le tunnel est démarré.
					</p>
				)}
			</div>
		</section>
	);
}
