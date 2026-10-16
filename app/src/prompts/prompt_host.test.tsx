import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { ModalProvider } from '@minimalstuff/ui';
import userEvent from '@testing-library/user-event';
import { act, screen, waitFor } from '@testing-library/react';

import { PromptHost } from '#/prompts/prompt_host';
import { useDaemonStore } from '#/state/daemon_store';
import { createFakeGateway } from '#/testing/fake_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { renderWithGateway } from '#/testing/render_with_gateway';
import { DaemonCommandError } from '#/daemon/daemon_command_error';
import type { CredentialRequestDto, EventMessage } from '#/daemon/protocol';

const passphraseRequest: CredentialRequestDto = {
	kind: 'passphrase',
	alias: 'nova',
	keyPath: '~/.ssh/id_ed25519',
};

const credentialEvent = (
	promptId: string,
	request: CredentialRequestDto
): EventMessage => ({
	event: 'credential.request',
	data: { promptId, ...request },
});

const hostKeyEvent = (promptId: string): EventMessage => ({
	event: 'hostkey.request',
	data: {
		kind: 'prompt',
		promptId,
		alias: 'nova',
		lookupName: '[nova.example.com]:2222',
		algorithm: 'ssh-ed25519',
		fingerprintSha256: 'SHA256:abcdef',
	},
});

const alertEvent: EventMessage = {
	event: 'hostkey.request',
	data: {
		kind: 'alert',
		alias: 'nova',
		presentedFingerprint: 'SHA256:new',
		reason: {
			kind: 'changed',
			knownFingerprint: 'SHA256:known',
			path: '/home/jules/.ssh/known_hosts',
			line: 12,
		},
	},
};

const findSecretInput = () =>
	screen.findByLabelText('Phrase de passe', { selector: 'input' });

function renderHost(gateway = createFakeGateway()) {
	const rendered = renderWithGateway(
		<>
			<ModalProvider />
			<PromptHost />
		</>,
		gateway
	);
	const receive = (event: EventMessage) =>
		act(() => useDaemonStore.getState().applyEvent(event));
	return { ...rendered, receive };
}

describe('PromptHost credential prompts', () => {
	it('should ask for the passphrase with the key and alias', async () => {
		const { receive } = renderHost();

		receive(credentialEvent('p1', passphraseRequest));

		expect(
			await screen.findByRole('dialog', { name: 'Phrase de passe' })
		).toHaveTextContent(
			'La clé ~/.ssh/id_ed25519 est chiffrée (connexion à nova).'
		);
	});

	it('should focus the secret input', async () => {
		const { receive } = renderHost();

		receive(credentialEvent('p1', passphraseRequest));

		expect(await findSecretInput()).toHaveFocus();
	});

	it('should hide the typed secret', async () => {
		const { receive } = renderHost();

		receive(credentialEvent('p1', passphraseRequest));

		expect(await findSecretInput()).toHaveAttribute('type', 'password');
	});

	it('should send the passphrase on Enter and dismiss the prompt', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));

		await user.type(await findSecretInput(), 's3cret{Enter}');

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith('p1', 's3cret')
		);
		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
		expect(useDaemonStore.getState().prompts).toEqual([]);
	});

	it('should send the passphrase with the submit button', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));

		await user.type(await findSecretInput(), 'abc');
		await user.click(screen.getByRole('button', { name: 'Valider' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith('p1', 'abc')
		);
	});

	it('should send null when cancelled with the button', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));

		await user.click(await screen.findByRole('button', { name: 'Annuler' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith('p1', null)
		);
	});

	it('should send null when closed with Escape', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));
		await screen.findByRole('dialog');

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith('p1', null)
		);
		expect(gateway.respondCredential).toHaveBeenCalledOnce();
	});

	it('should describe a password request', async () => {
		const { receive } = renderHost();

		receive(
			credentialEvent('p1', {
				kind: 'password',
				alias: 'nova',
				user: 'jules',
				host: 'nova.example.com',
			})
		);

		expect(
			await screen.findByRole('dialog', { name: 'Mot de passe' })
		).toHaveTextContent('Mot de passe de jules@nova.example.com (nova).');
	});

	it('should send keyboard-interactive answers in prompt order', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(
			credentialEvent('p1', {
				kind: 'keyboardInteractive',
				alias: 'nova',
				name: 'Duo',
				instructions: 'Entrez vos codes',
				prompts: [
					{ text: 'Code', echo: true },
					{ text: 'PIN', echo: false },
				],
			})
		);

		await user.type(await screen.findByLabelText('Code'), '123');
		await user.type(screen.getByLabelText('PIN'), '9876');
		await user.click(screen.getByRole('button', { name: 'Valider' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith('p1', [
				'123',
				'9876',
			])
		);
	});

	it('should show the instructions and echo only echoing prompts', async () => {
		const { receive } = renderHost();

		receive(
			credentialEvent('p1', {
				kind: 'keyboardInteractive',
				alias: 'nova',
				name: 'Duo',
				instructions: 'Entrez vos codes',
				prompts: [
					{ text: 'Code', echo: true },
					{ text: 'PIN', echo: false },
				],
			})
		);

		expect(await screen.findByText('Entrez vos codes')).toBeInTheDocument();
		expect(screen.getByLabelText('Code')).toHaveAttribute('type', 'text');
		expect(screen.getByLabelText('PIN')).toHaveAttribute('type', 'password');
	});

	it('should only offer to continue without prompts', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(
			credentialEvent('p1', {
				kind: 'keyboardInteractive',
				alias: 'nova',
				name: '',
				instructions: '',
				prompts: [],
			})
		);

		expect(
			await screen.findByRole('dialog', { name: 'Authentification nova' })
		).toBeInTheDocument();
		await user.click(screen.getByRole('button', { name: 'Continuer' }));

		await waitFor(() =>
			expect(gateway.respondCredential).toHaveBeenCalledWith('p1', [])
		);
	});

	it('should stay silent when the prompt is no longer pending', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.respondCredential.mockRejectedValueOnce(
			new DaemonCommandError('prompt_not_pending', 'plus en attente')
		);
		const { receive } = renderHost(gateway);
		receive(credentialEvent('p1', passphraseRequest));

		await user.type(await findSecretInput(), 'x{Enter}');

		await waitFor(() => expect(useDaemonStore.getState().prompts).toEqual([]));
		expect(useToastStore.getState().toasts).toEqual([]);
	});

	it('should toast other errors and still dismiss the prompt', async () => {
		const user = userEvent.setup();
		const gateway = createFakeGateway();
		gateway.respondCredential.mockRejectedValueOnce(
			new DaemonCommandError('io_error', 'canal fermé')
		);
		const { receive } = renderHost(gateway);
		receive(credentialEvent('p1', passphraseRequest));

		await user.type(await findSecretInput(), 'x{Enter}');

		await waitFor(() =>
			expect(useToastStore.getState().toasts[0]?.message).toBe(
				'Réponse impossible : canal fermé'
			)
		);
		expect(useDaemonStore.getState().prompts).toEqual([]);
	});

	it('should close the dialog without answering when another client resolved it', async () => {
		const { receive, gateway } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));
		await screen.findByRole('dialog');

		receive({ event: 'prompt.resolved', data: { promptId: 'p1' } });

		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
		expect(gateway.respondCredential).not.toHaveBeenCalled();
	});

	it('should show the next prompt once the first one is answered', async () => {
		const user = userEvent.setup();
		const { receive } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));
		receive(hostKeyEvent('p2'));
		await user.type(await findSecretInput(), 'x{Enter}');

		expect(
			await screen.findByRole('dialog', { name: 'Hôte inconnu' })
		).toBeInTheDocument();
	});

	it('should show one dialog at a time', async () => {
		const { receive } = renderHost();

		receive(credentialEvent('p1', passphraseRequest));
		receive(hostKeyEvent('p2'));

		expect(await screen.findAllByRole('dialog')).toHaveLength(1);
	});

	it('should have no axe violations', async () => {
		const { receive } = renderHost();
		receive(credentialEvent('p1', passphraseRequest));
		await screen.findByRole('dialog');

		expect(await axe(document.body)).toHaveNoViolations();
	});
});

describe('PromptHost host key prompts', () => {
	it('should show the key details and the verification advice', async () => {
		const { receive } = renderHost();

		receive(hostKeyEvent('p1'));

		const dialog = await screen.findByRole('dialog', { name: 'Hôte inconnu' });
		for (const text of [
			'nova',
			'[nova.example.com]:2222',
			'ssh-ed25519',
			'SHA256:abcdef',
			"Vérifiez cette empreinte auprès de l'administrateur du serveur avant de l'accepter.",
		]) {
			expect(dialog).toHaveTextContent(text, { normalizeWhitespace: true });
		}
	});

	it.each([
		['Accepter et enregistrer', 'acceptAndSave'],
		['Accepter une fois', 'acceptOnce'],
		['Refuser', 'reject'],
	])('should send the decision of "%s"', async (label, decision) => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(hostKeyEvent('p1'));

		await user.click(await screen.findByRole('button', { name: label }));

		await waitFor(() =>
			expect(gateway.respondHostKey).toHaveBeenCalledWith('p1', decision)
		);
		expect(useDaemonStore.getState().prompts).toEqual([]);
	});

	it('should reject on Escape', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(hostKeyEvent('p1'));
		await screen.findByRole('dialog');

		await user.keyboard('{Escape}');

		await waitFor(() =>
			expect(gateway.respondHostKey).toHaveBeenCalledWith('p1', 'reject')
		);
	});

	it('should offer to copy the fingerprint', async () => {
		const { receive } = renderHost();

		receive(hostKeyEvent('p1'));

		expect(
			await screen.findByRole('button', { name: "Copier l'empreinte" })
		).toBeInTheDocument();
	});

	it('should have no axe violations', async () => {
		const { receive } = renderHost();
		receive(hostKeyEvent('p1'));
		await screen.findByRole('dialog');

		expect(await axe(document.body)).toHaveNoViolations();
	});
});

describe('PromptHost host key alerts', () => {
	it('should explain the changed key and how to fix it', async () => {
		const { receive } = renderHost();

		receive(alertEvent);

		const dialog = await screen.findByRole('dialog', {
			name: "Clé d'hôte modifiée",
		});
		for (const text of [
			'Connexion à nova refusée.',
			'SHA256:new',
			'SHA256:known',
			'/home/jules/.ssh/known_hosts:12',
			'supprimez la ligne 12 de /home/jules/.ssh/known_hosts puis réessayez.',
		]) {
			expect(dialog).toHaveTextContent(text, { normalizeWhitespace: true });
		}
	});

	it('should not offer to accept the key', async () => {
		const { receive } = renderHost();

		receive(alertEvent);
		await screen.findByRole('dialog');

		expect(
			screen.getAllByRole('button').map((button) => button.textContent)
		).toEqual(["J'ai compris"]);
	});

	it('should not close on Escape', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(alertEvent);
		await screen.findByRole('dialog');

		await user.keyboard('{Escape}');

		expect(screen.getByRole('dialog')).toBeInTheDocument();
		expect(gateway.respondHostKey).not.toHaveBeenCalled();
	});

	it('should close on acknowledgement without answering the daemon', async () => {
		const user = userEvent.setup();
		const { receive, gateway } = renderHost();
		receive(alertEvent);

		await user.click(
			await screen.findByRole('button', { name: "J'ai compris" })
		);

		await waitFor(() =>
			expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
		);
		expect(useDaemonStore.getState().prompts).toEqual([]);
		expect(gateway.respondHostKey).not.toHaveBeenCalled();
	});

	it('should have no axe violations', async () => {
		const { receive } = renderHost();
		receive(alertEvent);
		await screen.findByRole('dialog');

		expect(await axe(document.body)).toHaveNoViolations();
	});
});
