import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';

import type { TunnelStateDto } from '#/daemon/protocol';
import { TunnelStatusBadge } from '#/shared/status_badge';

const STATES: readonly [TunnelStateDto, string][] = [
	[{ kind: 'stopped' }, 'Arrêté'],
	[{ kind: 'starting' }, 'Démarrage…'],
	[{ kind: 'active' }, 'Actif'],
	[
		{ kind: 'reconnecting', attempt: 2, nextRetryAt: 0 },
		'Reconnexion (tentative 2)',
	],
	[
		{
			kind: 'failed',
			reason: { kind: 'portInUse', address: '127.0.0.1:3333' },
		},
		'Échec',
	],
	[{ kind: 'external', owner: { pid: 4242, process: 'ssh' } }, 'Externe'],
	[{ kind: 'conflict' }, 'Conflit'],
];

const iconClassOf = (container: HTMLElement) =>
	container.querySelector('[aria-hidden="true"]')?.className;

describe('TunnelStatusBadge', () => {
	it.each(STATES)('should show the text of state %j', (state, label) => {
		render(<TunnelStatusBadge state={state} />);

		expect(screen.getByText(label)).toBeInTheDocument();
	});

	it('should show the failure reason as visible text', () => {
		render(
			<TunnelStatusBadge
				state={{
					kind: 'failed',
					reason: { kind: 'portInUse', address: '127.0.0.1:3333' },
				}}
			/>
		);

		expect(
			screen.getByText('Port déjà utilisé (127.0.0.1:3333)')
		).toBeVisible();
	});

	it('should show the owner of an external tunnel as visible text', () => {
		render(
			<TunnelStatusBadge
				state={{ kind: 'external', owner: { pid: 4242, process: 'ssh' } }}
			/>
		);

		expect(screen.getByText('fourni par ssh (pid 4242)')).toBeVisible();
	});

	it('should show the holder of the port on a conflict', () => {
		render(
			<TunnelStatusBadge
				state={{
					kind: 'conflict',
					owner: { pid: 4242, process: 'node.exe' },
				}}
			/>
		);

		expect(
			screen.getByText('port occupé par node.exe (pid 4242)')
		).toBeVisible();
	});

	it('should hide the icon from assistive technologies', () => {
		const { container } = render(
			<TunnelStatusBadge state={{ kind: 'active' }} />
		);

		expect(container.querySelector('[aria-hidden="true"]')).toBeInTheDocument();
	});

	it('should use a distinct icon for every state', () => {
		const icons = STATES.map(([state]) => {
			const { container, unmount } = render(
				<TunnelStatusBadge state={state} />
			);
			const icon = iconClassOf(container);
			unmount();
			return icon;
		});

		expect(new Set(icons).size).toBe(STATES.length);
	});

	it.each(STATES)('should have no axe violations for %j', async (state) => {
		const { container } = render(<TunnelStatusBadge state={state} />);

		expect(await axe(container)).toHaveNoViolations();
	});
});
