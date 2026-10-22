import { i18n } from '@lingui/core';
import { describe, expect, it } from 'vitest';

import type { CredentialRequestDto } from '#/daemon/protocol';
import {
	buildCredentialAnswer,
	getCredentialDescription,
	getCredentialFields,
	getCredentialSubmitLabel,
	getCredentialTitle,
} from '#/prompts/credential_prompt_text';

const passphrase: CredentialRequestDto = {
	kind: 'passphrase',
	alias: 'nova',
	keyPath: '~/.ssh/id_ed25519',
};
const password: CredentialRequestDto = {
	kind: 'password',
	alias: 'nova',
	user: 'jules',
	host: 'nova.example.com',
};
const keyboardInteractive: CredentialRequestDto = {
	kind: 'keyboardInteractive',
	alias: 'nova',
	name: '',
	instructions: 'One-time code',
	prompts: [
		{ text: 'Code', echo: true },
		{ text: 'PIN', echo: false },
	],
};

describe('credential prompt text', () => {
	it('should title a passphrase request', () => {
		expect(getCredentialTitle(passphrase, i18n)).toBe('Passphrase');
	});

	it('should describe a passphrase request with the key and alias', () => {
		expect(getCredentialDescription(passphrase, i18n)).toBe(
			'The key ~/.ssh/id_ed25519 is encrypted (connecting to nova).'
		);
	});

	it('should title a password request', () => {
		expect(getCredentialTitle(password, i18n)).toBe('Password');
	});

	it('should describe a password request with user, host and alias', () => {
		expect(getCredentialDescription(password, i18n)).toBe(
			'Password for jules@nova.example.com (nova).'
		);
	});

	it('should title a keyboard-interactive request after the alias when unnamed', () => {
		expect(getCredentialTitle(keyboardInteractive, i18n)).toBe(
			'Authentication for nova'
		);
	});

	it('should title a keyboard-interactive request with its name', () => {
		expect(
			getCredentialTitle({ ...keyboardInteractive, name: 'Duo' } as const, i18n)
		).toBe('Duo');
	});

	it('should describe a keyboard-interactive request with its instructions', () => {
		expect(getCredentialDescription(keyboardInteractive, i18n)).toBe(
			'One-time code'
		);
	});

	it('should hide only the inputs that do not echo', () => {
		expect(
			getCredentialFields(keyboardInteractive, i18n).map(
				(field) => field.isSecret
			)
		).toEqual([false, true]);
	});

	it('should offer a single secret field for a passphrase', () => {
		expect(getCredentialFields(passphrase, i18n)).toEqual([
			{ id: 'secret', label: 'Passphrase', isSecret: true },
		]);
	});

	it('should label the submit button "Continue" without prompts', () => {
		expect(
			getCredentialSubmitLabel(
				{ ...keyboardInteractive, prompts: [] } as const,
				i18n
			)
		).toBe('Continue');
	});

	it('should answer a passphrase with a string', () => {
		expect(buildCredentialAnswer(passphrase, ['secret'])).toBe('secret');
	});

	it('should answer keyboard-interactive with the answers in order', () => {
		expect(buildCredentialAnswer(keyboardInteractive, ['1', '2'])).toEqual([
			'1',
			'2',
		]);
	});
});
