import { describe, expect, it } from 'vitest';

import type { LogEntryDto } from '#/daemon/protocol';
import {
	appendLogEntry,
	MAXIMUM_LOG_ENTRIES,
	mergeLogEntries,
} from '#/tunnel_details/log_entries';

const buildEntry = (at: number, message = `message ${at}`): LogEntryDto => ({
	at,
	level: 'info',
	message,
});

describe('appendLogEntry', () => {
	it('should add the entry last', () => {
		expect(appendLogEntry([buildEntry(1)], buildEntry(2))).toEqual([
			buildEntry(1),
			buildEntry(2),
		]);
	});

	it('should drop the oldest entries beyond the maximum', () => {
		const entries = Array.from({ length: MAXIMUM_LOG_ENTRIES }, (_, index) =>
			buildEntry(index)
		);

		const appended = appendLogEntry(entries, buildEntry(MAXIMUM_LOG_ENTRIES));

		expect(appended).toHaveLength(MAXIMUM_LOG_ENTRIES);
		expect(appended.at(0)).toEqual(buildEntry(1));
		expect(appended.at(-1)).toEqual(buildEntry(MAXIMUM_LOG_ENTRIES));
	});
});

describe('mergeLogEntries', () => {
	it('should put the live entries after the history', () => {
		expect(mergeLogEntries([buildEntry(1)], [buildEntry(2)])).toEqual([
			buildEntry(1),
			buildEntry(2),
		]);
	});

	it('should skip a live entry already in the history', () => {
		expect(
			mergeLogEntries(
				[buildEntry(1), buildEntry(2)],
				[buildEntry(2), buildEntry(3)]
			)
		).toEqual([buildEntry(1), buildEntry(2), buildEntry(3)]);
	});

	it('should keep only the newest entries', () => {
		const fetched = Array.from({ length: MAXIMUM_LOG_ENTRIES }, (_, index) =>
			buildEntry(index)
		);

		const merged = mergeLogEntries(fetched, [buildEntry(MAXIMUM_LOG_ENTRIES)]);

		expect(merged).toHaveLength(MAXIMUM_LOG_ENTRIES);
		expect(merged.at(-1)).toEqual(buildEntry(MAXIMUM_LOG_ENTRIES));
	});
});
