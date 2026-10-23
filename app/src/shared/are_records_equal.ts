/** Whether both records hold the same keys with the same primitive values. */
export function areRecordsEqual(
	first: Readonly<Record<string, unknown>>,
	second: Readonly<Record<string, unknown>>
): boolean {
	const firstEntries = Object.entries(first);
	if (firstEntries.length !== Object.keys(second).length) return false;
	return firstEntries.every(([key, value]) => Object.is(second[key], value));
}
