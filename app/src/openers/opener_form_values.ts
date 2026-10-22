import type { OpenerSpecDto } from '#/daemon/protocol';

/** The parameters of an argument, replaced by the daemon when it runs the application. */
export const PLACEHOLDER_TEXTS = {
	hostPlaceholder: '{host}',
	portPlaceholder: '{port}',
	urlPlaceholder: '{url}',
} as const;

const ARGUMENT_SEPARATOR = '\n';
const PORT_SEPARATOR = ', ';
const NON_SLUG_PATTERN = /[^a-z0-9]+/gu;
const EDGE_DASHES_PATTERN = /^-+|-+$/gu;
const COMBINING_MARKS_PATTERN = /\p{M}/gu;

/** The form edits arguments one per line and ports as comma-separated text. */
export type OpenerFormValues = {
	name: string;
	id: string;
	command: string;
	args: string;
	ports: string;
};

export const EMPTY_OPENER_FORM_VALUES: OpenerFormValues = {
	name: '',
	id: '',
	command: '',
	args: '',
	ports: '',
};

export function toOpenerFormValues(
	opener: Readonly<OpenerSpecDto>
): OpenerFormValues {
	return {
		name: opener.name,
		id: opener.id,
		command: opener.command,
		args: opener.args.join(ARGUMENT_SEPARATOR),
		ports: opener.ports.join(PORT_SEPARATOR),
	};
}

/** An identifier proposed from a name: lowercase letters, digits and dashes. */
export function slugifyName(name: string): string {
	return name
		.normalize('NFD')
		.replaceAll(COMBINING_MARKS_PATTERN, '')
		.toLowerCase()
		.replaceAll(NON_SLUG_PATTERN, '-')
		.replaceAll(EDGE_DASHES_PATTERN, '');
}

/** One argument per non-blank line, each kept as typed: no shell will split or unquote it. */
export function parseArguments(text: string): string[] {
	return text.split(/\r?\n/u).filter((line) => line.trim() !== '');
}

export function parsePortList(text: string): number[] {
	return text
		.split(',')
		.map((part) => part.trim())
		.filter((part) => part !== '')
		.map(Number);
}

/** Builds the spec of valid form values. */
export function toOpenerSpec(
	values: Readonly<OpenerFormValues>
): OpenerSpecDto {
	return {
		id: values.id.trim(),
		name: values.name.trim(),
		command: values.command.trim(),
		args: parseArguments(values.args),
		ports: parsePortList(values.ports),
	};
}
