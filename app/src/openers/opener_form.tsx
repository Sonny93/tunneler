import { Button, Input, Textarea } from '@minimalstuff/ui';
import { useId, useState, type ChangeEvent, type FormEvent } from 'react';

import type { OpenerSpecDto } from '#/daemon/protocol';
import { FormErrorSummary } from '#/shared/form_error_summary';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import {
	getFirstInvalidOpenerField,
	validateOpenerForm,
	type OpenerFormErrors,
} from '#/openers/validate_opener_form';
import {
	EMPTY_OPENER_FORM_VALUES,
	slugifyName,
	toOpenerFormValues,
	toOpenerSpec,
	type OpenerFormValues,
} from '#/openers/opener_form_values';

const ARGUMENTS_HINT =
	'Un argument par ligne, transmis tels quels : aucun shell n’interprète les guillemets ni les espaces. {host}, {port} et {url} sont remplacés.';
const PORTS_HINT =
	'Séparés par des virgules. Le tunnel dont le port cible en fait partie propose cette application par défaut.';

type OpenerFormProps = {
	/** The application being edited, or `null` to add one. */
	opener: Readonly<OpenerSpecDto> | null;
	/** The identifiers of the user's other applications. */
	takenIds: readonly string[];
	onSubmit: (spec: OpenerSpecDto) => Promise<void>;
	onCancel: () => void;
};

function focusField(form: HTMLFormElement, fieldName: string): void {
	const field = form.elements.namedItem(fieldName);
	if (field instanceof HTMLElement) field.focus();
}

export function OpenerForm({
	opener,
	takenIds,
	onSubmit,
	onCancel,
}: Readonly<OpenerFormProps>) {
	const formId = useId();
	const isCreation = opener === null;
	const [values, setValues] = useState<OpenerFormValues>(
		opener ? toOpenerFormValues(opener) : EMPTY_OPENER_FORM_VALUES
	);
	const [isIdEdited, setIsIdEdited] = useState(false);
	const [errors, setErrors] = useState<OpenerFormErrors>({});
	const [isSaving, setIsSaving] = useState(false);
	const summary = useFormErrorSummary();

	const change = (field: keyof OpenerFormValues, value: string) => {
		setErrors(({ [field]: _edited, ...others }) => others);
		setValues((current) => ({ ...current, [field]: value }));
	};
	const handleName = (event: ChangeEvent<HTMLInputElement>) => {
		const name = event.target.value;
		change('name', name);
		if (isCreation && !isIdEdited) change('id', slugifyName(name));
	};
	const handleId = (event: ChangeEvent<HTMLInputElement>) => {
		setIsIdEdited(true);
		change('id', event.target.value);
	};
	const handleText =
		(field: 'command' | 'ports') => (event: ChangeEvent<HTMLInputElement>) =>
			change(field, event.target.value);
	const handleArguments = (event: ChangeEvent<HTMLTextAreaElement>) =>
		change('args', event.target.value);

	const submit = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (isSaving) return;
		const foundErrors = validateOpenerForm(values, takenIds);
		setErrors(foundErrors);
		const firstInvalidField = getFirstInvalidOpenerField(foundErrors);
		if (firstInvalidField !== null) {
			focusField(event.currentTarget, firstInvalidField);
			return;
		}
		summary.clearFailure();
		setIsSaving(true);
		void onSubmit(toOpenerSpec(values)).catch((rejection: unknown) => {
			setIsSaving(false);
			summary.reportFailure(rejection);
		});
	};

	return (
		<form
			id={formId}
			noValidate
			onSubmit={submit}
			aria-label={
				isCreation ? 'Ajouter une application' : `Modifier ${opener.name}`
			}
			className="flex flex-col gap-4 rounded-md border border-tn-border p-3"
		>
			<FormErrorSummary
				messages={summary.messages}
				attach={summary.attachSummary}
			/>
			<Input
				name="name"
				label="Nom"
				value={values.name}
				onChange={handleName}
				error={errors.name}
				required
				autoComplete="off"
			/>
			<Input
				name="id"
				label="Identifiant"
				value={values.id}
				onChange={handleId}
				error={errors.id}
				description={
					isCreation
						? 'Minuscules, chiffres et tirets. Non modifiable après la création.'
						: 'Non modifiable : les tunnels le désignent par cet identifiant.'
				}
				readOnly={!isCreation}
				required
				autoComplete="off"
			/>
			<Input
				name="command"
				label="Commande"
				value={values.command}
				onChange={handleText('command')}
				error={errors.command}
				description="Le programme à lancer, sans ses arguments."
				required
				autoComplete="off"
			/>
			<Textarea
				name="args"
				label="Arguments"
				value={values.args}
				onChange={handleArguments}
				error={errors.args}
				description={ARGUMENTS_HINT}
				rows={4}
			/>
			<Input
				name="ports"
				label="Ports suggérés"
				value={values.ports}
				onChange={handleText('ports')}
				error={errors.ports}
				description={PORTS_HINT}
				inputMode="numeric"
				autoComplete="off"
			/>
			<div className="flex justify-end gap-2">
				<Button variant="outline" color="neutral" onClick={onCancel}>
					Annuler
				</Button>
				<Button type="submit" loading={isSaving}>
					{isCreation ? 'Ajouter' : 'Enregistrer'}
				</Button>
			</div>
		</form>
	);
}
