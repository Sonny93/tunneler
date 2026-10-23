import type { I18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';

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
			return i18n._(msg`Passphrase`);
		case 'password':
			return i18n._(msg`Password`);
		case 'keyboardInteractive': {
			const { alias } = request;
			return request.name === ''
				? i18n._(msg`Authentication for ${alias}`)
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
			return i18n._(
				msg`The key ${keyPath} is encrypted (connecting to ${alias}).`
			);
		}
		case 'password': {
			const { user, host, alias } = request;
			return i18n._(msg`Password for ${user}@${host} (${alias}).`);
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
			return [{ id: 'secret', label: i18n._(msg`Passphrase`), isSecret: true }];
		case 'password':
			return [{ id: 'secret', label: i18n._(msg`Password`), isSecret: true }];
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
	return isWithoutInput ? i18n._(msg`Continue`) : i18n._(msg`Submit`);
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
		{ value: 'none', label: i18n._(msg`Don't remember`) },
		{ value: 'session', label: i18n._(msg`Until Tunneler closes`) },
	];
	if (!isKeyringEnabled) return options;
	return [
		...options,
		{ value: 'keyring', label: i18n._(msg`In the system keyring`) },
	];
}

export function toRemember(choice: RememberChoice): RememberDto | undefined {
	return choice === 'none' ? undefined : choice;
}
