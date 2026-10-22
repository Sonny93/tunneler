import { join, resolve } from 'node:path';
/**
 * One-off migration helper: fills the empty `msgstr` of `src/locales/fr/messages.po`
 * from JSON maps `{ "<english msgid>": "<french translation>" }`.
 *
 *   node scripts/fill_french_catalog.mjs <directory with the *.json maps>
 *
 * Exact `msgid` match only, an existing non-empty `msgstr` is kept. Prints the entries
 * that are still empty and the map keys that match no `msgid`.
 */
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';

const CATALOG_PATH = resolve(
	import.meta.dirname,
	'../src/locales/fr/messages.po'
);
const MESSAGE_FIELD_PATTERN = /^(msgid|msgstr) (".*")$/;
const CONTINUATION_PATTERN = /^(".*")$/;

const mapsDirectory = process.argv[2];
if (!mapsDirectory) {
	console.error('Usage: node scripts/fill_french_catalog.mjs <maps directory>');
	process.exit(1);
}

function loadTranslations(directory) {
	const translations = new Map();
	for (const fileName of readdirSync(directory).filter((name) =>
		name.endsWith('.json')
	)) {
		const entries = JSON.parse(readFileSync(join(directory, fileName), 'utf8'));
		for (const [source, translation] of Object.entries(entries)) {
			translations.set(source, translation);
		}
	}
	return translations;
}

function escapePo(text) {
	return text
		.replaceAll('\\', '\\\\')
		.replaceAll('"', '\\"')
		.replaceAll('\n', '\\n');
}

function parseBlock(lines) {
	const fields = { msgid: '', msgstr: '' };
	let currentField = null;
	for (const line of lines) {
		const field = MESSAGE_FIELD_PATTERN.exec(line);
		if (field) {
			currentField = field[1];
			fields[currentField] = JSON.parse(field[2]);
			continue;
		}
		const continuation = CONTINUATION_PATTERN.exec(line);
		if (continuation && currentField)
			fields[currentField] += JSON.parse(continuation[1]);
	}
	return fields;
}

function isEntryBlock(lines) {
	return (
		lines.some((line) => line.startsWith('msgid ')) &&
		parseBlock(lines).msgid !== ''
	);
}

function withMsgstr(lines, translation) {
	const msgstrStart = lines.findIndex((line) => line.startsWith('msgstr '));
	return [...lines.slice(0, msgstrStart), `msgstr "${escapePo(translation)}"`];
}

const translations = loadTranslations(resolve(mapsDirectory));
const usedSources = new Set();
const stillEmpty = [];

const blocks = readFileSync(CATALOG_PATH, 'utf8')
	.split('\n\n')
	.map((block) => {
		const lines = block.split('\n');
		if (!isEntryBlock(lines)) return block;
		const { msgid, msgstr } = parseBlock(lines);
		if (msgstr !== '') return block;
		const translation = translations.get(msgid);
		if (translation === undefined) {
			stillEmpty.push(msgid);
			return block;
		}
		usedSources.add(msgid);
		return withMsgstr(lines, translation).join('\n');
	});

writeFileSync(CATALOG_PATH, blocks.join('\n\n'));

const unusedSources = [...translations.keys()].filter(
	(source) => !usedSources.has(source)
);
console.log(`Filled ${usedSources.size} entries.`);
if (unusedSources.length > 0) {
	console.log(
		`\nMap keys matching no empty fr entry (${unusedSources.length}):`
	);
	for (const source of unusedSources) console.log(`  ${source}`);
}
console.log(`\nfr entries still empty (${stillEmpty.length}):`);
for (const msgid of stillEmpty) console.log(`  ${msgid}`);
