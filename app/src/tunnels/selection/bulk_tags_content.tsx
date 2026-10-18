import {
	useId,
	useRef,
	useState,
	type ChangeEvent,
	type FormEvent,
} from 'react';
import {
	Button,
	Input,
	ModalFooter,
	RadioOptions,
	Select,
} from '@minimalstuff/ui';

import { parseTags } from '#/tunnel_form/parse_tags';
import type { TagChange } from '#/tunnels/selection/tag_change';

type TagMode = 'add' | 'remove';

const MISSING_TAG_ERROR = 'Saisissez un tag.';

type BulkTagsContentProps = {
	/** The tags carried by at least one selected tunnel. */
	presentTags: readonly string[];
	/** Every tag of the configuration, suggested when adding. */
	knownTags: readonly string[];
	onApply: (change: TagChange) => void;
	onCancel: () => void;
};

function isTagMode(value: string): value is TagMode {
	return value === 'add' || value === 'remove';
}

/** Adds one tag to, or removes one tag from, the selected tunnels. */
export function BulkTagsContent({
	presentTags,
	knownTags,
	onApply,
	onCancel,
}: Readonly<BulkTagsContentProps>) {
	const formId = useId();
	const knownTagsId = useId();
	const addInputRef = useRef<HTMLInputElement>(null);
	const [mode, setMode] = useState<TagMode>('add');
	const [tagText, setTagText] = useState('');
	const [tagToRemove, setTagToRemove] = useState(presentTags[0] ?? '');
	const [error, setError] = useState<string | undefined>();
	const modeOptions = [
		{ value: 'add', label: 'Ajouter un tag' },
		{
			value: 'remove',
			label: 'Retirer un tag',
			disabled: presentTags.length === 0,
		},
	];

	const handleModeChange = (selected: string) => {
		if (isTagMode(selected)) setMode(selected);
	};
	const handleTextChange = (event: ChangeEvent<HTMLInputElement>) => {
		setTagText(event.target.value);
		setError(undefined);
	};
	const handleRemovedTagChange = (event: ChangeEvent<HTMLSelectElement>) =>
		setTagToRemove(event.target.value);

	const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (mode === 'remove') {
			onApply({ kind: 'remove', tag: tagToRemove });
			return;
		}
		const tags = parseTags(tagText);
		if (tags.length === 0) {
			setError(MISSING_TAG_ERROR);
			addInputRef.current?.focus();
			return;
		}
		onApply({ kind: 'add', tags });
	};

	return (
		<>
			<form id={formId} onSubmit={handleSubmit} className="flex flex-col gap-4">
				<RadioOptions
					name="tag-mode"
					label="Action"
					options={modeOptions}
					value={mode}
					onChange={handleModeChange}
				/>
				{mode === 'add' ? (
					<>
						<Input
							ref={addInputRef}
							label="Tag à ajouter"
							value={tagText}
							onChange={handleTextChange}
							error={error}
							list={knownTagsId}
							autoComplete="off"
						/>
						<datalist id={knownTagsId}>
							{knownTags.map((tag) => (
								<option key={tag} value={tag}>
									{tag}
								</option>
							))}
						</datalist>
					</>
				) : (
					<Select
						label="Tag à retirer"
						options={presentTags.map((tag) => ({ value: tag, label: tag }))}
						value={tagToRemove}
						onChange={handleRemovedTagChange}
					/>
				)}
			</form>
			<ModalFooter>
				<Button variant="outline" color="neutral" onClick={onCancel}>
					Annuler
				</Button>
				<Button type="submit" form={formId}>
					Appliquer
				</Button>
			</ModalFooter>
		</>
	);
}
