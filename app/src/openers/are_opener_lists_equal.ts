import type { OpenerSpecDto } from '#/daemon/protocol';

function areListsEqual<TItem>(
	first: readonly TItem[],
	second: readonly TItem[]
): boolean {
	return (
		first.length === second.length &&
		first.every((item, index) => item === second[index])
	);
}

function areOpenersEqual(
	first: Readonly<OpenerSpecDto>,
	second: Readonly<OpenerSpecDto>
): boolean {
	return (
		first.id === second.id &&
		first.name === second.name &&
		first.command === second.command &&
		areListsEqual(first.args, second.args) &&
		areListsEqual(first.ports, second.ports)
	);
}

/** Whether both lists hold the same applications in the same order. */
export function areOpenerListsEqual(
	first: readonly Readonly<OpenerSpecDto>[],
	second: readonly Readonly<OpenerSpecDto>[]
): boolean {
	return (
		first.length === second.length &&
		first.every((opener, index) => {
			const other = second[index];
			return other !== undefined && areOpenersEqual(opener, other);
		})
	);
}
