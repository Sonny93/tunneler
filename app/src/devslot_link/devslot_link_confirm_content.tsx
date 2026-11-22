import { useLingui } from '@lingui/react/macro';
import { Button, ModalFooter } from '@minimalstuff/ui';
import { useEffect, useRef, type RefObject } from 'react';

import {
	useOpenDevslotInstance,
	type OpenPhase,
} from '#/devslot_link/use_open_devslot_instance';
import { useUiStore } from '#/state/ui_store';

type DevslotLinkConfirmContentProps = {
	requestId: number;
	host: string;
	instance: string;
	/** Attached to "Cancel", the safe default: the modal focuses it when it opens. */
	cancelButtonRef: RefObject<HTMLButtonElement | null>;
};

type FailedPhase = Extract<OpenPhase, { kind: 'failed' }>;

function Failure({ phase }: Readonly<{ phase: FailedPhase }>) {
	return (
		<div role="alert" className="flex flex-col gap-2">
			{phase.summary !== null && <p className="m-0">{phase.summary}</p>}
			<ul className="m-0 flex list-disc flex-col gap-1 pl-5">
				{phase.problems.map((problem) => (
					<li key={problem}>{problem}</li>
				))}
			</ul>
		</div>
	);
}

function Question({
	host,
	instance,
	isRunning,
}: Readonly<{ host: string; instance: string; isRunning: boolean }>) {
	const { t } = useLingui();

	return (
		<div className="flex flex-col gap-2">
			<p className="m-0">{t`A link asks Tunneler to open the instance ${instance} on the host ${host}.`}</p>
			<p className="m-0">
				{t`Tunneler will connect to the host over SSH, run ds ls --json, start tunnels for the web ports of the instance (temporary, unless your devslot setting saves them) and open the application in your browser.`}
			</p>
			<p className="m-0">{t`Only continue if you trust where this link comes from.`}</p>
			{isRunning && <p className="m-0">{t`Opening ${instance} on ${host}…`}</p>}
		</div>
	);
}

/** The question a valid link asks, its progress and its problems. */
export function DevslotLinkConfirmContent({
	requestId,
	host,
	instance,
	cancelButtonRef,
}: Readonly<DevslotLinkConfirmContentProps>) {
	const { t } = useLingui();
	const { phase, open } = useOpenDevslotInstance({
		requestId,
		host,
		instance,
	});
	const closeButtonRef = useRef<HTMLButtonElement>(null);
	const isRunning = phase.kind === 'running';
	const isFailed = phase.kind === 'failed';
	const close = () => useUiStore.getState().closeDevslotLink(requestId);
	const handleOpen = () => void open();

	useEffect(() => {
		if (isFailed) closeButtonRef.current?.focus();
	}, [isFailed]);

	return (
		<>
			<div aria-busy={isRunning}>
				{phase.kind === 'failed' ? (
					<Failure phase={phase} />
				) : (
					<Question host={host} instance={instance} isRunning={isRunning} />
				)}
			</div>
			<ModalFooter>
				{isFailed ? (
					<Button ref={closeButtonRef} onClick={close}>
						{t`Close`}
					</Button>
				) : (
					<>
						<Button
							ref={cancelButtonRef}
							variant="outline"
							color="neutral"
							disabled={isRunning}
							onClick={close}
						>
							{t`Cancel`}
						</Button>
						<Button loading={isRunning} onClick={handleOpen}>
							{t`Open`}
						</Button>
					</>
				)}
			</ModalFooter>
		</>
	);
}
