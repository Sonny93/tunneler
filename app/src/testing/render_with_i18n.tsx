import type { ReactElement } from 'react';
import { render, type RenderResult } from '@testing-library/react';

import { I18nTestProvider } from '#/testing/i18n_test_provider';

/** `render` inside an `I18nProvider`, for components that use Lingui but no daemon gateway. */
export function renderWithI18n(ui: ReactElement): RenderResult {
	return render(<I18nTestProvider>{ui}</I18nTestProvider>);
}
