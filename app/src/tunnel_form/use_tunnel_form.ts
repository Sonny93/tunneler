import { useLingui } from '@lingui/react/macro';
import { useReducer, useState, type FormEvent } from 'react';

import type {
	TunnelDefinitionDto,
	TunnelDto,
	TunnelSpecDto,
} from '#/daemon/protocol';
import { useDaemonGateway } from '#/daemon/use_daemon_gateway';
import { useToastStore } from '#/shared/toasts/toast_store';
import { useFormErrorSummary } from '#/shared/use_form_error_summary';
import { useUiStore } from '#/state/ui_store';
import { clearEditedFieldErrors } from '#/tunnel_form/clear_edited_field_errors';
import {
	toTemporaryTunnelSpec,
	toTunnelSpec,
} from '#/tunnel_form/to_tunnel_spec';
import {
	tunnelFormReducer,
	type TunnelFormAction,
} from '#/tunnel_form/tunnel_form_reducer';
import { toTunnelFormValues } from '#/tunnel_form/tunnel_form_values';
import {
	getFirstInvalidField,
	validateTunnelForm,
	type TunnelFormErrors,
} from '#/tunnel_form/validate_tunnel_form';
import { getTunnelEndpointLabel } from '#/tunnels/tunnel_endpoint_label';

/** A `temporary` form starts a tunnel that is never saved. */
export type TunnelFormMode = 'saved' | 'temporary';

type UseTunnelFormReturn = {
	values: ReturnType<typeof toTunnelFormValues>;
	errors: TunnelFormErrors;
	isSaving: boolean;
	summary: ReturnType<typeof useFormErrorSummary>;
	dispatch: (action: TunnelFormAction) => void;
	submit: (event: FormEvent<HTMLFormElement>) => void;
};

function focusField(form: HTMLFormElement, fieldName: string): void {
	const field = form.elements.namedItem(fieldName);
	if (field instanceof HTMLElement) field.focus();
}

export function useTunnelForm(
	tunnel: Readonly<TunnelDefinitionDto> | null,
	draft: Readonly<TunnelSpecDto> | null = null,
	mode: TunnelFormMode = 'saved'
): UseTunnelFormReturn {
	const { t } = useLingui();
	const source = tunnel ?? draft;
	const gateway = useDaemonGateway();
	const [values, dispatchToReducer] = useReducer(
		tunnelFormReducer,
		source,
		toTunnelFormValues
	);
	const [errors, setErrors] = useState<TunnelFormErrors>({});
	const [isSaving, setIsSaving] = useState(false);
	const summary = useFormErrorSummary();

	const dispatch = (action: TunnelFormAction) => {
		setErrors((current) => clearEditedFieldErrors(current, action, values));
		dispatchToReducer(action);
	};

	const submitToDaemon = (spec: TunnelSpecDto): Promise<TunnelDto> => {
		if (mode === 'temporary') {
			return gateway.startTemporaryTunnel(toTemporaryTunnelSpec(values));
		}
		return tunnel
			? gateway.updateTunnel({ id: tunnel.id, ...spec })
			: gateway.createTunnel(spec);
	};

	const describeOutcome = (spec: TunnelSpecDto): string => {
		const label = getTunnelEndpointLabel(spec);
		if (mode === 'temporary') return t`Temporary tunnel ${label} started`;
		if (tunnel) return t`Tunnel ${label} saved`;
		return t`Tunnel ${label} created`;
	};

	const save = async () => {
		const spec = toTunnelSpec(values);
		setIsSaving(true);
		summary.clearFailure();
		let savedTunnel: TunnelDto;
		try {
			savedTunnel = await submitToDaemon(spec);
		} catch (rejection) {
			setIsSaving(false);
			summary.reportFailure(rejection);
			return;
		}
		useToastStore
			.getState()
			.pushToast({ tone: 'success', message: describeOutcome(spec) });
		if (tunnel) useUiStore.getState().closePanel();
		else useUiStore.getState().closePanelFocusingTunnel(savedTunnel.id);
	};

	const submit = (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (isSaving) return;
		const foundErrors = validateTunnelForm(values);
		setErrors(foundErrors);
		const firstInvalidField = getFirstInvalidField(foundErrors);
		if (firstInvalidField !== null) {
			focusField(event.currentTarget, firstInvalidField);
			return;
		}
		void save();
	};

	return { values, errors, isSaving, summary, dispatch, submit };
}
