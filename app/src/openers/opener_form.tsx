import { useLingui } from '@lingui/react/macro';
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
	PLACEHOLDER_TEXTS,
	slugifyName,
	toOpenerFormValues,
	toOpenerSpec,
	type OpenerFormValues,
} from '#/openers/opener_form_values';

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
	const { t, i18n } = useLingui();
	const formId = useId();
	const { hostPlaceholder, portPlaceholder, urlPlaceholder } =
		PLACEHOLDER_TEXTS;
	const isCreation = opener === null;
	const openerName = opener?.name ?? '';
	const formLabel = isCreation ? t`Add an application` : t`Edit ${openerName}`;
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
			aria-label={formLabel}
			className="flex flex-col gap-4 rounded-md border border-tn-border p-3"
		>
			<FormErrorSummary
				messages={summary.messages}
				attach={summary.attachSummary}
			/>
			<Input
				name="name"
				label={t`Name`}
				value={values.name}
				onChange={handleName}
				error={errors.name && i18n._(errors.name)}
				required
				autoComplete="off"
			/>
			<Input
				name="id"
				label={t`Identifier`}
				value={values.id}
				onChange={handleId}
				error={errors.id && i18n._(errors.id)}
				description={
					isCreation
						? t`Lowercase letters, digits and dashes. Cannot be changed after creation.`
						: t`Cannot be changed: tunnels refer to it by this identifier.`
				}
				readOnly={!isCreation}
				required
				autoComplete="off"
			/>
			<Input
				name="command"
				label={t`Command`}
				value={values.command}
				onChange={handleText('command')}
				error={errors.command && i18n._(errors.command)}
				description={t`The program to run, without its arguments.`}
				required
				autoComplete="off"
			/>
			<Textarea
				name="args"
				label={t`Arguments`}
				value={values.args}
				onChange={handleArguments}
				error={errors.args && i18n._(errors.args)}
				description={t`One argument per line, passed as is: no shell interprets quotes or spaces. ${hostPlaceholder}, ${portPlaceholder} and ${urlPlaceholder} are replaced.`}
				rows={4}
			/>
			<Input
				name="ports"
				label={t`Suggested ports`}
				value={values.ports}
				onChange={handleText('ports')}
				error={errors.ports && i18n._(errors.ports)}
				description={t`Separated by commas. A tunnel whose target port is one of them offers this application by default.`}
				inputMode="numeric"
				autoComplete="off"
			/>
			<div className="flex justify-end gap-2">
				<Button variant="outline" color="neutral" onClick={onCancel}>
					{t`Cancel`}
				</Button>
				<Button type="submit" loading={isSaving}>
					{isCreation ? t`Add` : t`Save`}
				</Button>
			</div>
		</form>
	);
}
