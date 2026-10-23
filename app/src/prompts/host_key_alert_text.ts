import type { I18n } from '@lingui/core';
import { t } from '@lingui/core/macro';

import type { HostKeyAlertReasonDto } from '#/daemon/protocol';

export type HostKeyAlertFact = { label: string; value: string };

export function getHostKeyAlertTitle(
	reason: Readonly<HostKeyAlertReasonDto>,
	i18n: I18n
): string {
	switch (reason.kind) {
		case 'changed':
			return t(i18n)`Host key changed`;
		case 'revoked':
			return t(i18n)`Host key revoked`;
		case 'unknownRejectedByPolicy':
			return t(i18n)`Unknown host rejected`;
	}
}

export function getHostKeyAlertFacts(
	reason: Readonly<HostKeyAlertReasonDto>,
	presentedFingerprint: string,
	i18n: I18n
): HostKeyAlertFact[] {
	const presented = {
		label: t(i18n)`Presented fingerprint`,
		value: presentedFingerprint,
	};
	switch (reason.kind) {
		case 'changed':
			return [
				presented,
				{
					label: t(i18n)`Stored fingerprint`,
					value: reason.knownFingerprint,
				},
				{ label: t(i18n)`Location`, value: `${reason.path}:${reason.line}` },
			];
		case 'revoked':
			return [
				presented,
				{ label: t(i18n)`Location`, value: `${reason.path}:${reason.line}` },
			];
		case 'unknownRejectedByPolicy':
			return [presented];
	}
}

export function getHostKeyAlertAdvice(
	reason: Readonly<HostKeyAlertReasonDto>,
	i18n: I18n
): string | null {
	switch (reason.kind) {
		case 'changed': {
			const { line, path } = reason;
			return t(
				i18n
			)`If the change is legitimate, delete line ${line} of ${path} and try again.`;
		}
		case 'revoked':
			return null;
		case 'unknownRejectedByPolicy':
			return t(
				i18n
			)`StrictHostKeyChecking forbids accepting an unknown key for this host.`;
	}
}
