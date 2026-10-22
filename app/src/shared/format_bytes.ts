import { i18n } from '@lingui/core';
import { msg } from '@lingui/core/macro';
import type { MessageDescriptor } from '@lingui/core';

const UNITS: readonly MessageDescriptor[] = [msg`B`, msg`KB`, msg`MB`, msg`GB`];
const BYTES_PER_UNIT = 1024;
const MAXIMUM_FRACTION_DIGITS = 1;

/** A size in the units and number format of the active language. */
export function formatBytes(bytes: number): string {
	const numberFormat = new Intl.NumberFormat(i18n.locale, {
		maximumFractionDigits: MAXIMUM_FRACTION_DIGITS,
	});
	let value = bytes;
	let unitIndex = 0;
	while (value >= BYTES_PER_UNIT && unitIndex < UNITS.length - 1) {
		value /= BYTES_PER_UNIT;
		unitIndex += 1;
	}
	const unit = UNITS[unitIndex];
	return `${numberFormat.format(value)} ${unit ? i18n._(unit) : ''}`;
}
