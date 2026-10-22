import type { RefObject } from 'react';
import { useLingui } from '@lingui/react/macro';
import { Button, CopyButton, ModalFooter } from '@minimalstuff/ui';

import type { PendingPrompt } from '#/state/daemon_state';
import type { HostKeyDecisionDto } from '#/daemon/protocol';
import { useToastStore } from '#/shared/toasts/toast_store';

type HostKeyPrompt = Extract<PendingPrompt, { kind: 'hostKey' }>;

type HostKeyPromptContentProps = {
	prompt: HostKeyPrompt;
	/** Attached to "Reject", the safe default: the modal focuses it when it opens. */
	rejectButtonRef: RefObject<HTMLButtonElement | null>;
	onDecide: (decision: HostKeyDecisionDto) => void;
};

type CopyRenderProps = { copied: boolean; copy: () => void | Promise<void> };

function CopyFingerprintButton({ copied, copy }: Readonly<CopyRenderProps>) {
	const { t } = useLingui();
	const handleClick = () => void copy();

	return (
		<Button variant="outline" color="neutral" size="sm" onClick={handleClick}>
			{copied ? t`Fingerprint copied` : t`Copy the fingerprint`}
		</Button>
	);
}

export function HostKeyPromptContent({
	prompt,
	rejectButtonRef,
	onDecide,
}: Readonly<HostKeyPromptContentProps>) {
	const { t } = useLingui();
	const reject = () => onDecide('reject');
	const acceptOnce = () => onDecide('acceptOnce');
	const acceptAndSave = () => onDecide('acceptAndSave');
	const reportCopyFailure = () =>
		useToastStore.getState().pushToast({
			tone: 'error',
			message: t`Couldn't copy the fingerprint`,
		});

	return (
		<>
			<dl className="mb-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
				<dt className="font-medium">{t`Alias`}</dt>
				<dd>{prompt.alias}</dd>
				<dt className="font-medium">{t`Looked-up name`}</dt>
				<dd>{prompt.lookupName}</dd>
				<dt className="font-medium">{t`Algorithm`}</dt>
				<dd>{prompt.algorithm}</dd>
				<dt className="font-medium">{t`SHA-256 fingerprint`}</dt>
				<dd className="font-mono select-all break-all">
					{prompt.fingerprintSha256}
				</dd>
			</dl>
			<CopyButton value={prompt.fingerprintSha256} onError={reportCopyFailure}>
				{CopyFingerprintButton}
			</CopyButton>
			<p className="mt-4 text-sm">
				{t`Check this fingerprint with the server administrator before accepting it.`}
			</p>
			<ModalFooter>
				<Button
					ref={rejectButtonRef}
					variant="outline"
					color="neutral"
					onClick={reject}
				>
					{t`Reject`}
				</Button>
				<Button variant="outline" onClick={acceptOnce}>
					{t`Accept once`}
				</Button>
				<Button onClick={acceptAndSave}>{t`Accept and save`}</Button>
			</ModalFooter>
		</>
	);
}
