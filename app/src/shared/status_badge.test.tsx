import { screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { axe } from 'vitest-axe';

import type { TunnelStateDto } from '#/daemon/protocol';
import { TunnelStatusBadge } from '#/shared/status_badge';
import { renderWithI18n } from '#/testing/render_with_i18n';

const STATES: readonly [TunnelStateDto, string][] = [
	[{ kind: 'stopped' }, 'Stopped'],
	[{ kind: 'standby' }, 'Standby'],
	[{ kind: 'starting' }, 'Starting…'],
	[{ kind: 'active' }, 'Active'],
	[
		{ kind: 'reconnecting', attempt: 2, nextRetryAt: 0 },
		'Reconnecting (attempt 2)',
	],
	[
		{
			kind: 'failed',
			reason: { kind: 'portInUse', address: '127.0.0.1:3333' },
		},
		'Failed',
	],
	[{ kind: 'external', owner: { pid: 4242, process: 'ssh' } }, 'External'],
	[{ kind: 'conflict' }, 'Conflict'],
];

const iconClassOf = (container: HTMLElement) =>
	container.querySelector('[aria-hidden="true"]')?.className;

describe('TunnelStatusBadge', () => {
	it.each(STATES)('should show the text of state %j', (state, label) => {
		renderWithI18n(<TunnelStatusBadge state={state} />);

		expect(screen.getByText(label)).toBeInTheDocument();
	});

	it('should show the failure reason as visible text', () => {
		renderWithI18n(
			<TunnelStatusBadge
				state={{
					kind: 'failed',
					reason: { kind: 'portInUse', address: '127.0.0.1:3333' },
				}}
			/>
		);

		expect(
			screen.getByText('Port already in use (127.0.0.1:3333)')
		).toBeVisible();
	});

	it('should show the last wake-up failure of a standby tunnel as visible text', () => {
		renderWithI18n(
			<TunnelStatusBadge
				state={{
					kind: 'standby',
					lastFailure: { kind: 'connect', message: 'authentication refused' },
				}}
			/>
		);

		expect(
			screen.getByText(
				/last wake-up failed: SSH connection failed: authentication refused/
			)
		).toBeVisible();
	});

	it('should show the owner of an external tunnel as visible text', () => {
		renderWithI18n(
			<TunnelStatusBadge
				state={{ kind: 'external', owner: { pid: 4242, process: 'ssh' } }}
			/>
		);

		expect(screen.getByText('provided by ssh (pid 4242)')).toBeVisible();
	});

	it('should show the holder of the port on a conflict', () => {
		renderWithI18n(
			<TunnelStatusBadge
				state={{
					kind: 'conflict',
					owner: { pid: 4242, process: 'node.exe' },
				}}
			/>
		);

		expect(
			screen.getByText('port in use by node.exe (pid 4242)')
		).toBeVisible();
	});

	it('should hide the icon from assistive technologies', () => {
		const { container } = renderWithI18n(
			<TunnelStatusBadge state={{ kind: 'active' }} />
		);

		expect(container.querySelector('[aria-hidden="true"]')).toBeInTheDocument();
	});

	it('should use a distinct icon for every state', () => {
		const icons = STATES.map(([state]) => {
			const { container, unmount } = renderWithI18n(
				<TunnelStatusBadge state={state} />
			);
			const icon = iconClassOf(container);
			unmount();
			return icon;
		});

		expect(new Set(icons).size).toBe(STATES.length);
	});

	it.each(STATES)('should have no axe violations for %j', async (state) => {
		const { container } = renderWithI18n(<TunnelStatusBadge state={state} />);

		expect(await axe(container)).toHaveNoViolations();
	});
});
