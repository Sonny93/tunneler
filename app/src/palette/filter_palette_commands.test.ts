import { describe, expect, it } from 'vitest';

import {
	filterPaletteCommands,
	MAXIMUM_PALETTE_RESULTS,
} from '#/palette/filter_palette_commands';
import type { PaletteCommand } from '#/palette/palette_command';

const buildCommand = (
	label: string,
	category: PaletteCommand['category'] = 'Tunnel'
): PaletteCommand => ({
	id: label,
	label,
	category,
	action: { kind: 'newTunnel' },
});

const COMMANDS = [
	buildCommand('Start Adonis'),
	buildCommand('Stop Adonis'),
	buildCommand('Configuration of nova', 'Host'),
	buildCommand('Open settings', 'General'),
	buildCommand('Open café'),
];

const labelsOf = (commands: readonly PaletteCommand[]) =>
	commands.map((command) => command.label);

describe('filterPaletteCommands', () => {
	it('should keep every command for a blank query', () => {
		expect(filterPaletteCommands(COMMANDS, '   ')).toEqual(COMMANDS);
	});

	it('should ignore case', () => {
		expect(labelsOf(filterPaletteCommands(COMMANDS, 'ADONIS'))).toEqual([
			'Start Adonis',
			'Stop Adonis',
		]);
	});

	it('should ignore accents on both sides', () => {
		expect(labelsOf(filterPaletteCommands(COMMANDS, 'cafe'))).toEqual([
			'Open café',
		]);
		expect(labelsOf(filterPaletteCommands(COMMANDS, 'CAFÉ'))).toEqual([
			'Open café',
		]);
	});

	it('should require every word', () => {
		expect(labelsOf(filterPaletteCommands(COMMANDS, 'adonis stop'))).toEqual([
			'Stop Adonis',
		]);
	});

	it('should match the category too', () => {
		expect(labelsOf(filterPaletteCommands(COMMANDS, 'host'))).toEqual([
			'Configuration of nova',
		]);
	});

	it('should cap the number of results', () => {
		const manyCommands = Array.from(
			{ length: MAXIMUM_PALETTE_RESULTS + 10 },
			(_, index) => buildCommand(`Command ${index}`)
		);

		expect(filterPaletteCommands(manyCommands, '')).toHaveLength(
			MAXIMUM_PALETTE_RESULTS
		);
	});
});
