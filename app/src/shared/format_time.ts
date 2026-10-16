const timeFormat = new Intl.DateTimeFormat('fr-FR', {
	hour: '2-digit',
	minute: '2-digit',
	second: '2-digit',
	hourCycle: 'h23',
});

/** Local time as HH:MM:SS from epoch milliseconds. */
export function formatTime(epochMilliseconds: number): string {
	return timeFormat.format(new Date(epochMilliseconds));
}
