import { render, type RenderResult } from '@testing-library/react';
import type { ReactElement } from 'react';

import { DaemonGatewayProvider } from '#/daemon/daemon_gateway_context';
import { AppUIProvider } from '#/shared/app_ui_provider';
import { createFakeGateway, type FakeGateway } from '#/testing/fake_gateway';
import { I18nTestProvider } from '#/testing/i18n_test_provider';

type RenderWithGatewayResult = RenderResult & { gateway: FakeGateway };

/**
 * Renders `ui` inside an `I18nTestProvider`, a `DaemonGatewayProvider` and the `AppUIProvider` (French lib labels). It does not mount the lib's
 * `ModalProvider` (`App` mounts it once): tests of components that open modals
 * render `<ModalProvider />` next to the component under test.
 */
export function renderWithGateway(
	ui: ReactElement,
	gateway: FakeGateway = createFakeGateway()
): RenderWithGatewayResult {
	const result = render(
		<I18nTestProvider>
			<DaemonGatewayProvider gateway={gateway}>
				<AppUIProvider>{ui}</AppUIProvider>
			</DaemonGatewayProvider>
		</I18nTestProvider>
	);
	return { ...result, gateway };
}
