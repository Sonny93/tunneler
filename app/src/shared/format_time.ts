import { i18n } from '@lingui/core';

/** Local time as HH:MM:SS from epoch milliseconds. */
export function formatTime(epochMilliseconds: number): string {
	const timeFormat = new Intl.DateTimeFormat(i18n.locale, {
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit',
		hourCycle: 'h23',
	});
	return timeFormat.format(new Date(epochMilliseconds));
}
