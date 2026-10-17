import type { LogEntryDto } from '#/daemon/protocol';

export const MAXIMUM_LOG_ENTRIES = 500;

function keepNewest(entries: readonly LogEntryDto[]): LogEntryDto[] {
	return entries.slice(-MAXIMUM_LOG_ENTRIES);
}

/** Adds an entry at the end and drops the oldest ones beyond the maximum. */
export function appendLogEntry(
	entries: readonly LogEntryDto[],
	entry: Readonly<LogEntryDto>
): LogEntryDto[] {
	return keepNewest([...entries, entry]);
}

function getEntryKey(entry: Readonly<LogEntryDto>): string {
	return `${entry.at}|${entry.level}|${entry.message}`;
}

/**
 * The fetched history followed by the live entries it does not already
 * contain (an entry can arrive both in the history and as an event).
 */
export function mergeLogEntries(
	fetched: readonly LogEntryDto[],
	live: readonly LogEntryDto[]
): LogEntryDto[] {
	const fetchedKeys = new Set(fetched.map(getEntryKey));
	const missingLiveEntries = live.filter(
		(entry) => !fetchedKeys.has(getEntryKey(entry))
	);
	return keepNewest([...fetched, ...missingLiveEntries]);
}
