import { vi } from 'vitest';

/** Runs before test_setup.ts: the UI library reads matchMedia while it is imported. */
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
