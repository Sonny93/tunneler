const UNITS: readonly string[] = ['o', 'Ko', 'Mo', 'Go'];
const BYTES_PER_UNIT = 1024;
const MAXIMUM_FRACTION_DIGITS = 1;

const numberFormat = new Intl.NumberFormat('fr-FR', {
	maximumFractionDigits: MAXIMUM_FRACTION_DIGITS,
});

export function formatBytes(bytes: number): string {
	let value = bytes;
	let unitIndex = 0;
	while (value >= BYTES_PER_UNIT && unitIndex < UNITS.length - 1) {
		value /= BYTES_PER_UNIT;
		unitIndex += 1;
	}
	return `${numberFormat.format(value)} ${UNITS[unitIndex]}`;
}
