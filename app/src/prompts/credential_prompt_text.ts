import { t } from '@lingui/core/macro';
import type { I18n } from '@lingui/core';

import type { CredentialRequestDto, RememberDto } from '#/daemon/protocol';

export type CredentialField = {
	id: string;
	label: string;
	isSecret: boolean;
};

export function getCredentialTitle(
	request: Readonly<CredentialRequestDto>,
	i18n: I18n
): string {
	switch (request.kind) {
		case 'passphrase':
			return t(i18n)`Passphrase`;
		case 'password':
			return t(i18n)`Password`;
		case 'keyboardInteractive': {
			const { alias } = request;
			return request.name === ''
				? t(i18n)`Authentication for ${alias}`
				: request.name;
		}
	}
}

/** The explanation shown above the inputs; empty when there is nothing to add. */
export function getCredentialDescription(
	request: Readonly<CredentialRequestDto>,
	i18n: I18n
): string {
	switch (request.kind) {
		case 'passphrase': {
			const { keyPath, alias } = request;
			return t(i18n)`The key ${keyPath} is encrypted (connecting to ${alias}).`;
		}
		case 'password': {
			const { user, host, alias } = request;
			return t(i18n)`Password for ${user}@${host} (${alias}).`;
		}
		case 'keyboardInteractive':
			return request.instructions;
	}
}

export function getCredentialFields(
	request: Readonly<CredentialRequestDto>,
	i18n: I18n
): CredentialField[] {
	switch (request.kind) {
		case 'passphrase':
			return [{ id: 'secret', label: t(i18n)`Passphrase`, isSecret: true }];
		case 'password':
			return [{ id: 'secret', label: t(i18n)`Password`, isSecret: true }];
		case 'keyboardInteractive':
			return request.prompts.map((prompt, position) => ({
				id: `prompt-${position}`,
				label: prompt.text,
				isSecret: !prompt.echo,
			}));
	}
}

export function getCredentialSubmitLabel(
	request: Readonly<CredentialRequestDto>,
	i18n: I18n
): string {
	const isWithoutInput =
		request.kind === 'keyboardInteractive' && request.prompts.length === 0;
	return isWithoutInput ? t(i18n)`Continue` : t(i18n)`Submit`;
}

/** A string for single-secret requests, the answers in prompt order for keyboard-interactive. */
export function buildCredentialAnswer(
	request: Readonly<CredentialRequestDto>,
	answers: readonly string[]
): string | string[] {
	if (request.kind === 'keyboardInteractive') return [...answers];
	return answers[0] ?? '';
}

export type RememberChoice = 'none' | RememberDto;

export const DEFAULT_REMEMBER_CHOICE: RememberChoice = 'session';

export type RememberOption = { value: RememberChoice; label: string };

/** Only passphrases can be remembered. */
export function canRemember(request: Readonly<CredentialRequestDto>): boolean {
	return request.kind === 'passphrase';
}

/** The system keyring option is offered only when the settings enable it. */
export function getRememberOptions(
	isKeyringEnabled: boolean,
	i18n: I18n
): RememberOption[] {
	const options: RememberOption[] = [
		{ value: 'none', label: t(i18n)`Don't remember` },
		{ value: 'session', label: t(i18n)`Until Tunneler closes` },
	];
	if (!isKeyringEnabled) return options;
	return [
		...options,
		{ value: 'keyring', label: t(i18n)`In the system keyring` },
	];
}

export function toRemember(choice: RememberChoice): RememberDto | undefined {
	return choice === 'none' ? undefined : choice;
}
