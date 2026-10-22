import { useId } from 'react';
import { Button, Menu, MenuItem } from '@minimalstuff/ui';

import { useUiStore } from '#/state/ui_store';
import type { TunnelDto } from '#/daemon/protocol';
import { copyTextWithToast } from '#/shared/copy_text';
import { isTunnelRunning } from '#/tunnels/is_tunnel_running';
import { useSendToStandby } from '#/standby/use_send_to_standby';
import { useOpenTunnel } from '#/tunnel_details/use_open_tunnel';
import { useCheckTarget } from '#/tunnel_details/use_check_target';
import { useTunnelActions } from '#/tunnels/list/use_tunnel_actions';
import { buildSshCommand } from '#/tunnel_details/build_ssh_command';

type TunnelActionsSectionProps = {
	tunnel: TunnelDto;
};

export function TunnelActionsSection({
	tunnel,
}: Readonly<TunnelActionsSectionProps>) {
	const headingId = useId();
	const commandId = useId();
	const runningReasonId = useId();
	const restartReasonId = useId();
	const openTunnelEditor = useUiStore((state) => state.openTunnelEditor);
	const { restart, stopAndRemove, persist } = useTunnelActions(tunnel);
	const { sendTunnel } = useSendToStandby();
	const { isOpenable, label, alternatives, open, openWith } =
		useOpenTunnel(tunnel);
	const isRunning = isTunnelRunning(tunnel.state);
	const { isChecking, check } = useCheckTarget(tunnel);
	const hasTarget = tunnel.kind !== 'dynamic';
	const command = buildSshCommand(tunnel);

	const handleCopy = () => void copyTextWithToast(command, 'Commande copiée');
	const handleEdit = () => openTunnelEditor(tunnel.id);
	const handleStandby = () => void sendTunnel(tunnel);

	return (
		<section aria-labelledby={headingId} className="flex flex-col gap-3">
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				Actions
			</h3>
			{hasTarget && (
				<div className="flex flex-col gap-1">
					<div className="flex flex-wrap gap-2">
						{isOpenable && (
							<Button
								startIcon="i-mdi-open-in-new"
								variant="outline"
								color="neutral"
								onClick={open}
							>
								{label}
							</Button>
						)}
						{isOpenable && alternatives.length > 0 && (
							<Menu
								trigger={
									<Button
										startIcon="i-mdi-open-in-app"
										variant="outline"
										color="neutral"
									>
										Ouvrir avec…
									</Button>
								}
							>
								{alternatives.map((choice) => (
									<MenuItem key={choice.id} onClick={() => openWith(choice.id)}>
										{choice.name}
									</MenuItem>
								))}
							</Menu>
						)}
						<Button
							startIcon="i-mdi-stethoscope"
							variant="outline"
							color="neutral"
							loading={isChecking}
							disabled={!isRunning}
							aria-describedby={isRunning ? undefined : runningReasonId}
							onClick={check}
						>
							Tester la cible
						</Button>
					</div>
					{!isRunning && (
						<p id={runningReasonId} className="m-0 text-xs text-tn-muted">
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
					{tunnel.isAwake && (
						<Button
							startIcon="i-mdi-sleep"
							variant="outline"
							color="neutral"
							onClick={handleStandby}
						>
							Mettre en veille
						</Button>
					)}
					{tunnel.isTemporary ? (
						<>
							<Button
								startIcon="i-mdi-content-save-outline"
								variant="outline"
								color="neutral"
								onClick={persist}
							>
								Enregistrer le tunnel
							</Button>
							<Button
								startIcon="i-mdi-stop"
								variant="outline"
								color="danger"
								onClick={stopAndRemove}
							>
								Arrêter et retirer
							</Button>
						</>
					) : (
						<Button
							startIcon="i-mdi-pencil-outline"
							variant="outline"
							color="neutral"
							onClick={handleEdit}
						>
							Modifier
						</Button>
					)}
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
