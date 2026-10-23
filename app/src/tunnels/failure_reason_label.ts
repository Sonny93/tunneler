import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { FailureReasonDto } from '#/daemon/protocol';

const MILLISECONDS_PER_SECOND = 1000;

export function getFailureReasonLabel(
	reason: Readonly<FailureReasonDto>,
	i18n: I18n
): string {
	switch (reason.kind) {
		case 'portInUse': {
			const { address } = reason;
			return i18n._(msg`Port already in use (${address})`);
		}
		case 'listenFailed': {
			const { address, message } = reason;
			return i18n._(msg`Cannot listen on ${address}: ${message}`);
		}
		case 'connect': {
			const { message } = reason;
			return i18n._(msg`SSH connection failed: ${message}`);
		}
		case 'remoteForwardRefused': {
			const { address } = reason;
			return i18n._(msg`The server refuses to listen on ${address}`);
		}
		case 'agentUnresponsive': {
			const timeoutSeconds = Math.round(
				reason.timeoutMs / MILLISECONDS_PER_SECOND
			);
			return i18n._(
				msg`The SSH agent is not responding (no answer in ${timeoutSeconds} s): is it waiting for a confirmation?`
			);
		}
	}
}
