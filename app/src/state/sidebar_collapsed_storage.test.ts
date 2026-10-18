import { afterEach, describe, expect, it, vi } from 'vitest';

import {
	readSidebarCollapsed,
	writeSidebarCollapsed,
} from '#/state/sidebar_collapsed_storage';

describe('sidebar collapsed storage', () => {
	afterEach(() => vi.restoreAllMocks());

	it('should default to expanded', () => {
		expect(readSidebarCollapsed()).toBe(false);
	});

	it('should remember a collapsed sidebar', () => {
		writeSidebarCollapsed(true);

		expect(readSidebarCollapsed()).toBe(true);
	});

	it('should remember an expanded sidebar', () => {
		writeSidebarCollapsed(true);
		writeSidebarCollapsed(false);

		expect(readSidebarCollapsed()).toBe(false);
	});

	it('should read as expanded when the storage throws', () => {
		vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
			throw new Error('blocked');
		});

		expect(readSidebarCollapsed()).toBe(false);
	});

	it('should not throw when writing to a blocked storage', () => {
		vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
			throw new Error('blocked');
		});

		expect(() => writeSidebarCollapsed(true)).not.toThrow();
	});
});
