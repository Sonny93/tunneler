import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';

import type { FailureReasonDto } from '#/daemon/protocol';

const MILLISECONDS_PER_SECOND = 1000;

export function getFailureReasonLabel(
	reason: Readonly<FailureReasonDto>,
	i18n: I18n
): string {
	switch (reason.kind) {
		case 'portInUse': {
			const { address } = reason;
			return t(i18n)`Port already in use (${address})`;
		}
		case 'listenFailed': {
			const { address, message } = reason;
			return t(i18n)`Cannot listen on ${address}: ${message}`;
		}
		case 'connect': {
			const { message } = reason;
			return t(i18n)`SSH connection failed: ${message}`;
		}
		case 'remoteForwardRefused': {
			const { address } = reason;
			return t(i18n)`The server refuses to listen on ${address}`;
		}
		case 'agentUnresponsive': {
			const timeoutSeconds = Math.round(
				reason.timeoutMs / MILLISECONDS_PER_SECOND
			);
			return t(
				i18n
			)`The SSH agent is not responding (no answer in ${timeoutSeconds} s): is it waiting for a confirmation?`;
		}
	}
}
