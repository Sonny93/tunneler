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
	instructions: 'Code à usage unique',
	prompts: [
		{ text: 'Code', echo: true },
		{ text: 'PIN', echo: false },
	],
};

describe('credential prompt text', () => {
	it('should title a passphrase request', () => {
		expect(getCredentialTitle(passphrase)).toBe('Phrase de passe');
	});

	it('should describe a passphrase request with the key and alias', () => {
		expect(getCredentialDescription(passphrase)).toBe(
			'La clé ~/.ssh/id_ed25519 est chiffrée (connexion à nova).'
		);
	});

	it('should title a password request', () => {
		expect(getCredentialTitle(password)).toBe('Mot de passe');
	});

	it('should describe a password request with user, host and alias', () => {
		expect(getCredentialDescription(password)).toBe(
			'Mot de passe de jules@nova.example.com (nova).'
		);
	});

	it('should title a keyboard-interactive request after the alias when unnamed', () => {
		expect(getCredentialTitle(keyboardInteractive)).toBe(
			'Authentification nova'
		);
	});

	it('should title a keyboard-interactive request with its name', () => {
		expect(
			getCredentialTitle({ ...keyboardInteractive, name: 'Duo' } as const)
		).toBe('Duo');
	});

	it('should describe a keyboard-interactive request with its instructions', () => {
		expect(getCredentialDescription(keyboardInteractive)).toBe(
			'Code à usage unique'
		);
	});

	it('should hide only the inputs that do not echo', () => {
		expect(
			getCredentialFields(keyboardInteractive).map((field) => field.isSecret)
		).toEqual([false, true]);
	});

	it('should offer a single secret field for a passphrase', () => {
		expect(getCredentialFields(passphrase)).toEqual([
			{ id: 'secret', label: 'Phrase de passe', isSecret: true },
		]);
	});

	it('should label the submit button "Continuer" without prompts', () => {
		expect(
			getCredentialSubmitLabel({ ...keyboardInteractive, prompts: [] } as const)
		).toBe('Continuer');
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
