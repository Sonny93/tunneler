import '@minimalstuff/ui/style.css';
import 'virtual:uno.css';

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from '#/app';
import { createTauriGateway } from '#/daemon/tauri_gateway';
import { DaemonGatewayProvider } from '#/daemon/daemon_gateway_context';

const gateway = createTauriGateway();

const rootElement = document.getElementById('root');
if (!rootElement) throw new Error('Root element #root not found');

createRoot(rootElement).render(
	<StrictMode>
		<DaemonGatewayProvider gateway={gateway}>
			<App />
		</DaemonGatewayProvider>
	</StrictMode>
);
