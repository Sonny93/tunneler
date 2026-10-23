import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

import type { HostKeyAlertReasonDto } from '#/daemon/protocol';

export type HostKeyAlertFact = { label: string; value: string };

export function getHostKeyAlertTitle(
	reason: Readonly<HostKeyAlertReasonDto>,
	i18n: I18n
): string {
	switch (reason.kind) {
		case 'changed':
			return i18n._(msg`Host key changed`);
		case 'revoked':
			return i18n._(msg`Host key revoked`);
		case 'unknownRejectedByPolicy':
			return i18n._(msg`Unknown host rejected`);
	}
}

export function getHostKeyAlertFacts(
	reason: Readonly<HostKeyAlertReasonDto>,
	presentedFingerprint: string,
	i18n: I18n
): HostKeyAlertFact[] {
	const presented = {
		label: i18n._(msg`Presented fingerprint`),
		value: presentedFingerprint,
	};
	switch (reason.kind) {
		case 'changed':
			return [
				presented,
				{
					label: i18n._(msg`Stored fingerprint`),
					value: reason.knownFingerprint,
				},
				{
					label: i18n._(msg`Location`),
					value: `${reason.path}:${reason.line}`,
				},
			];
		case 'revoked':
			return [
				presented,
				{
					label: i18n._(msg`Location`),
					value: `${reason.path}:${reason.line}`,
				},
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
			return i18n._(
				msg`If the change is legitimate, delete line ${line} of ${path} and try again.`
			);
		}
		case 'revoked':
			return null;
		case 'unknownRejectedByPolicy':
			return i18n._(
				msg`StrictHostKeyChecking forbids accepting an unknown key for this host.`
			);
	}
}
