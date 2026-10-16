import { afterEach, expect, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import * as axeMatchers from 'vitest-axe/matchers';
import * as jestDomMatchers from '@testing-library/jest-dom/matchers';

expect.extend(jestDomMatchers);
expect.extend(axeMatchers);

vi.stubGlobal(
	'matchMedia',
	(query: string): MediaQueryList =>
		({
			matches: false,
			media: query,
			addEventListener: vi.fn(),
			removeEventListener: vi.fn(),
		}) as unknown as MediaQueryList
);

afterEach(() => {
	cleanup();
	localStorage.clear();
});
