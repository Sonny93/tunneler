import { useLingui } from '@lingui/react/macro';
import { Button } from '@minimalstuff/ui';
import { useId } from 'react';

import type { AvailableOpenerDto, OpenerSpecDto } from '#/daemon/protocol';
import { DocumentationHint } from '#/openers/documentation_hint';
import { OpenerForm } from '#/openers/opener_form';
import { useOpenerFocus } from '#/openers/use_opener_focus';
import type { UseOpenersDraftReturn } from '#/openers/use_openers_draft';

function describeCommandLine(opener: Readonly<OpenerSpecDto>): string {
	return [opener.command, ...opener.args]
		.map((part) => (/\s/u.test(part) ? `"${part}"` : part))
		.join(' ');
}

type OpenerListProps<TOpener extends OpenerSpecDto> = {
	headingId: string;
	openers: readonly TOpener[];
	children?: (opener: Readonly<TOpener>) => React.ReactNode;
};

function OpenerList<TOpener extends OpenerSpecDto>({
	headingId,
	openers,
	children,
}: Readonly<OpenerListProps<TOpener>>) {
	return (
		<ul
			aria-labelledby={headingId}
			className="m-0 flex list-none flex-col gap-2 p-0"
		>
			{openers.map((opener) => (
				<li
					key={opener.id}
					className="flex flex-wrap items-center justify-between gap-2"
				>
					<div className="min-w-0">
						<div className="text-sm font-medium">{opener.name}</div>
						<code className="block break-all font-mono text-xs text-tn-muted">
							{describeCommandLine(opener)}
						</code>
					</div>
					{children?.(opener)}
				</li>
			))}
		</ul>
	);
}

type UserOpenerActionsProps = {
	opener: Readonly<OpenerSpecDto>;
	onEdit: (id: string) => void;
	onRemove: (opener: Readonly<OpenerSpecDto>) => void;
};

function UserOpenerActions({
	opener,
	onEdit,
	onRemove,
}: Readonly<UserOpenerActionsProps>) {
	const { t } = useLingui();
	const { name } = opener;

	return (
		<div className="flex gap-1">
			<Button
				size="xs"
				variant="outline"
				color="neutral"
				aria-label={t`Edit ${name}`}
				data-opener-edit={opener.id}
				onClick={() => onEdit(opener.id)}
			>
				{t`Edit`}
			</Button>
			<Button
				size="xs"
				variant="outline"
				color="danger"
				aria-label={t`Delete ${name}`}
				onClick={() => onRemove(opener)}
			>
				{t`Delete`}
			</Button>
		</div>
	);
}

type DetectedOpenerActionsProps = {
	opener: Readonly<AvailableOpenerDto>;
	onCustomize: (opener: Readonly<AvailableOpenerDto>) => void;
};

function DetectedOpenerActions({
	opener,
	onCustomize,
}: Readonly<DetectedOpenerActionsProps>) {
	const { t } = useLingui();
	const { name } = opener;
	const handleCustomize = () => onCustomize(opener);

	return (
		<Button
			size="xs"
			variant="outline"
			color="neutral"
			aria-label={t`Customize ${name}`}
			onClick={handleCustomize}
		>
			{t`Customize`}
		</Button>
	);
}

type ApplicationsSectionProps = {
	draft: UseOpenersDraftReturn;
};

/** The "Applications" section of the settings: detected ones read-only, the user's ones edited in the draft. */
export function ApplicationsSection({
	draft,
}: Readonly<ApplicationsSectionProps>) {
	const { t } = useLingui();
	const headingId = useId();
	const detectedId = useId();
	const userId = useId();
	const { containerRef, requestFocus } = useOpenerFocus();
	const { editing, userOpeners, detectedOpeners } = draft;
	const prefilledSpec = editing.kind === 'create' ? editing.draft : undefined;
	const editedOpener =
		editing.kind === 'edit'
			? userOpeners.find(({ id }) => id === editing.id)
			: undefined;
	const takenIds = userOpeners
		.map(({ id }) => id)
		.filter((id) => id !== editedOpener?.id);

	const handleCreate = () => {
		draft.startCreating();
		requestFocus({ kind: 'form' });
	};
	const handleCustomize = (opener: Readonly<AvailableOpenerDto>) => {
		draft.startCustomizing(opener);
		requestFocus({ kind: 'form' });
	};
	const handleEdit = (id: string) => {
		draft.startEditing(id);
		requestFocus({ kind: 'form' });
	};
	const handleSubmit = (spec: OpenerSpecDto) => {
		draft.save(spec);
		requestFocus({ kind: 'edit', id: spec.id });
	};
	const handleCancel = () => {
		draft.stopEditing();
		requestFocus({ kind: 'add' });
	};
	const handleRemove = (opener: Readonly<OpenerSpecDto>) =>
		void draft.remove(opener).then((isRemoved) => {
			if (isRemoved) requestFocus({ kind: 'add' });
		});

	return (
		<section
			ref={containerRef}
			aria-labelledby={headingId}
			className="flex flex-col gap-4"
		>
			<h3 id={headingId} className="m-0 text-sm font-semibold">
				{t`Applications`}
			</h3>
			<p className="m-0 text-xs text-tn-muted">
				{t`They open a tunnel instead of the browser. Commands run without a shell, with the arguments as is.`}
			</p>
			<div className="flex flex-col gap-2">
				<h4 id={detectedId} className="m-0 text-xs font-semibold">
					{t`Detected on this computer`}
				</h4>
				{detectedOpeners.length === 0 ? (
					<p className="m-0 text-xs text-tn-muted">
						{t`No application detected.`}
					</p>
				) : (
					<OpenerList headingId={detectedId} openers={detectedOpeners}>
						{(opener) => (
							<DetectedOpenerActions
								opener={opener}
								onCustomize={handleCustomize}
							/>
						)}
					</OpenerList>
				)}
				<DocumentationHint />
			</div>
			<div className="flex flex-col gap-2">
				<h4 id={userId} className="m-0 text-xs font-semibold">
					{t`Added`}
				</h4>
				{userOpeners.length === 0 ? (
					<p className="m-0 text-xs text-tn-muted">
						{t`No application added.`}
					</p>
				) : (
					<OpenerList headingId={userId} openers={userOpeners}>
						{(opener) => (
							<UserOpenerActions
								opener={opener}
								onEdit={handleEdit}
								onRemove={handleRemove}
							/>
						)}
					</OpenerList>
				)}
			</div>
			{editing.kind === 'none' && (
				<div>
					<Button
						startIcon="i-mdi-plus"
						variant="outline"
						color="neutral"
						data-opener-add
						onClick={handleCreate}
					>
						{t`Add an application`}
					</Button>
				</div>
			)}
			{editing.kind !== 'none' && (
				<OpenerForm
					key={editedOpener?.id ?? prefilledSpec?.id ?? 'new'}
					opener={editedOpener ?? null}
					draft={prefilledSpec}
					detectedOpeners={detectedOpeners}
					takenIds={takenIds}
					onSubmit={handleSubmit}
					onCancel={handleCancel}
					onInputChange={draft.reportFormInput}
				/>
			)}
		</section>
	);
}
