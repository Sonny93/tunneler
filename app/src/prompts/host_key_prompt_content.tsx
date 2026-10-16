import { Button, CopyButton, ModalFooter } from '@minimalstuff/ui';

import type { PendingPrompt } from '#/state/daemon_state';
import type { HostKeyDecisionDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';

type HostKeyPrompt = Extract<PendingPrompt, { kind: 'hostKey' }>;

type HostKeyPromptContentProps = {
	prompt: HostKeyPrompt;
	onDecide: (decision: HostKeyDecisionDto) => void;
};

const reportCopyFailure = () =>
	useToastStore
		.getState()
		.pushToast({ tone: 'error', message: "Copie de l'empreinte impossible" });

type CopyRenderProps = { copied: boolean; copy: () => void | Promise<void> };

function CopyFingerprintButton({ copied, copy }: Readonly<CopyRenderProps>) {
	const handleClick = () => void copy();

	return (
		<Button variant="outline" color="neutral" size="sm" onClick={handleClick}>
			{copied ? 'Empreinte copiée' : "Copier l'empreinte"}
		</Button>
	);
}

export function HostKeyPromptContent({
	prompt,
	onDecide,
}: Readonly<HostKeyPromptContentProps>) {
	const reject = () => onDecide('reject');
	const acceptOnce = () => onDecide('acceptOnce');
	const acceptAndSave = () => onDecide('acceptAndSave');

	return (
		<>
			<dl className="mb-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
				<dt className="font-medium">Alias</dt>
				<dd>{prompt.alias}</dd>
				<dt className="font-medium">Nom recherché</dt>
				<dd>{prompt.lookupName}</dd>
				<dt className="font-medium">Algorithme</dt>
				<dd>{prompt.algorithm}</dd>
				<dt className="font-medium">Empreinte SHA-256</dt>
				<dd className="font-mono select-all break-all">
					{prompt.fingerprintSha256}
				</dd>
			</dl>
			<CopyButton value={prompt.fingerprintSha256} onError={reportCopyFailure}>
				{CopyFingerprintButton}
			</CopyButton>
			<p className="mt-4 text-sm">
				Vérifiez cette empreinte auprès de l'administrateur du serveur avant de
				l'accepter.
			</p>
			<ModalFooter>
				<Button variant="outline" color="neutral" onClick={reject}>
					Refuser
				</Button>
				<Button variant="outline" onClick={acceptOnce}>
					Accepter une fois
				</Button>
				<Button onClick={acceptAndSave}>Accepter et enregistrer</Button>
			</ModalFooter>
		</>
	);
}
