import type { ReactElement } from 'react';
import { render, type RenderResult } from '@testing-library/react';

import { DaemonGatewayProvider } from '#/daemon/daemon_gateway_context';
import { createFakeGateway, type FakeGateway } from '#/testing/fake_gateway';

type RenderWithGatewayResult = RenderResult & { gateway: FakeGateway };

/**
 * Renders `ui` inside a `DaemonGatewayProvider`. It does not mount the lib's
 * `ModalProvider` (`App` mounts it once): tests of components that open modals
 * render `<ModalProvider />` next to the component under test.
 */
export function renderWithGateway(
	ui: ReactElement,
	gateway: FakeGateway = createFakeGateway()
): RenderWithGatewayResult {
	const result = render(
		<DaemonGatewayProvider gateway={gateway}>{ui}</DaemonGatewayProvider>
	);
	return { ...result, gateway };
}
