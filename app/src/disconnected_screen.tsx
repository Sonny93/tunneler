import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';

import type { DaemonCommandError } from '#/daemon/daemon_command_error';
import { describeDaemonError } from '#/daemon/describe_daemon_error';

type DisconnectedScreenProps = {
	error: Readonly<DaemonCommandError>;
	onRetry: () => void;
};

export function DisconnectedScreen({
	error,
	onRetry,
}: Readonly<DisconnectedScreenProps>) {
	const { t, i18n } = useLingui();
	const reason = describeDaemonError(error, i18n);

	return (
		<main className="flex flex-col items-start gap-3 p-4">
			<h1 className="text-lg font-semibold">Tunneler</h1>
			<p role="alert">{t`Daemon unreachable: ${reason}`}</p>
			<Button onClick={onRetry}>{t`Retry`}</Button>
		</main>
	);
}
