import { axe } from 'vitest-axe';
import { describe, expect, it } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { useUiStore } from '#/state/ui_store';
import { buildX11Anchor } from '#/testing/factories';
import { useDaemonStore } from '#/state/daemon_store';
import { X11HeaderControls } from '#/x11/x11_header_controls';
import { renderWithGateway } from '#/testing/render_with_gateway';

describe('X11HeaderControls', () => {
	it('should open the X11 drawer of the host from a named button', async () => {
		renderWithGateway(<X11HeaderControls host="nova" />);

		await userEvent.click(
			screen.getByRole('button', { name: 'X11 pour nova' })
		);

		expect(useUiStore.getState().panel).toEqual({ kind: 'x11', host: 'nova' });
	});

	it('should show no status while the anchor is stopped', () => {
		renderWithGateway(<X11HeaderControls host="nova" />);

		expect(screen.queryByText(/^X11 (actif|:)/)).not.toBeInTheDocument();
	});

	it('should show the display when the anchor is active', () => {
		useDaemonStore.setState({ x11Anchors: [buildX11Anchor()] });
		renderWithGateway(<X11HeaderControls host="nova" />);

		expect(screen.getByText('X11 actif : localhost:10.0')).toBeInTheDocument();
	});

	it('should ignore the anchors of other hosts', () => {
		useDaemonStore.setState({
			x11Anchors: [buildX11Anchor({ host: 'other' })],
		});
		renderWithGateway(<X11HeaderControls host="nova" />);

		expect(screen.queryByText(/localhost:10.0/)).not.toBeInTheDocument();
	});

	it('should have no axe violations', async () => {
		useDaemonStore.setState({ x11Anchors: [buildX11Anchor()] });
		renderWithGateway(
			<main>
				<X11HeaderControls host="nova" />
			</main>
		);

		expect(await axe(document.body)).toHaveNoViolations();
	});
});
