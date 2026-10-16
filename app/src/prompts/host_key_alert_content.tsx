import { Button, ModalFooter } from '@minimalstuff/ui';

import type { PendingPrompt } from '#/state/daemon_state';
import {
	getHostKeyAlertAdvice,
	getHostKeyAlertFacts,
} from '#/prompts/host_key_alert_text';

type HostKeyAlert = Extract<PendingPrompt, { kind: 'hostKeyAlert' }>;

type HostKeyAlertContentProps = {
	alert: HostKeyAlert;
	onAcknowledge: () => void;
};

export function HostKeyAlertContent({
	alert,
	onAcknowledge,
}: Readonly<HostKeyAlertContentProps>) {
	const facts = getHostKeyAlertFacts(alert.reason, alert.presentedFingerprint);
	const advice = getHostKeyAlertAdvice(alert.reason);

	return (
		<>
			<p className="mb-3">Connexion à {alert.alias} refusée.</p>
			<dl className="mb-3 flex flex-col gap-2">
				{facts.map((fact) => (
					<div key={fact.label}>
						<dt className="font-medium">{fact.label}</dt>
						<dd className="font-mono select-all break-all">{fact.value}</dd>
					</div>
				))}
			</dl>
			{advice && <p>{advice}</p>}
			<ModalFooter>
				<Button onClick={onAcknowledge}>J'ai compris</Button>
			</ModalFooter>
		</>
	);
}
