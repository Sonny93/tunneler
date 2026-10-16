import '@minimalstuff/ui/style.css';
import 'virtual:uno.css';

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from '#/app';

const rootElement = document.getElementById('root');
if (!rootElement) throw new Error('Root element #root not found');

createRoot(rootElement).render(
	<StrictMode>
		<App />
	</StrictMode>
);
