import { useCallback, useRef, useState } from 'react';

import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import {
	runManualUpdateCheck,
	type ManualUpdateCheckOutcome,
} from '#/updates/run_manual_update_check';

export type ManualUpdateCheckStatus =
	| { kind: 'idle' }
	| { kind: 'checking' }
	| Exclude<ManualUpdateCheckOutcome, { kind: 'skipped' }>;

type UseManualUpdateCheckReturn = {
	status: ManualUpdateCheckStatus;
	check: () => Promise<void>;
};

const IDLE: ManualUpdateCheckStatus = { kind: 'idle' };
const CHECKING: ManualUpdateCheckStatus = { kind: 'checking' };

/** Checks for a newer version on demand; a call during a check is ignored. */
export function useManualUpdateCheck(): UseManualUpdateCheckReturn {
	const gateway = useDaemonGateway();
	const [status, setStatus] = useState<ManualUpdateCheckStatus>(IDLE);
	const isCheckingRef = useRef(false);

	const check = useCallback(async () => {
		if (isCheckingRef.current) return;
		isCheckingRef.current = true;
		setStatus(CHECKING);
		const outcome = await runManualUpdateCheck(gateway);
		isCheckingRef.current = false;
		setStatus(outcome.kind === 'skipped' ? IDLE : outcome);
	}, [gateway]);

	return { status, check };
}
