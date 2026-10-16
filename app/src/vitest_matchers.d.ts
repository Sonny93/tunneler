import type { AxeMatchers } from 'vitest-axe/matchers';
import type { TestingLibraryMatchers } from '@testing-library/jest-dom/matchers';

declare module 'vitest' {
	// oxlint-disable-next-line typescript/no-explicit-any
	interface Assertion<T = any>
		extends TestingLibraryMatchers<unknown, T>, AxeMatchers {}
	interface AsymmetricMatchersContaining
		extends TestingLibraryMatchers<unknown, unknown>, AxeMatchers {}
}
