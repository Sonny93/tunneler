import type { OpenSchemeDto, TunnelKindDto } from '#/daemon/protocol';
import type {
	OpenHostChoice,
	TunnelFormValues,
} from '#/tunnel_form/tunnel_form_values';

export type TunnelFormTextField =
	| 'host'
	| 'listenPort'
	| 'targetHost'
	| 'listenAddress'
	| 'tags'
	| 'openPath';

export type TunnelFormAction =
	| { type: 'textChanged'; field: TunnelFormTextField; value: string }
	| { type: 'kindChanged'; kind: TunnelKindDto }
	| { type: 'openSchemeChanged'; scheme: OpenSchemeDto }
	| { type: 'openHostChanged'; host: OpenHostChoice }
	| { type: 'targetPortChanged'; value: string }
	| { type: 'targetPortRelinked' };

/** The target port follows the listen port until it is edited; relinking snaps it back. */
export function tunnelFormReducer(
	values: Readonly<TunnelFormValues>,
	action: Readonly<TunnelFormAction>
): TunnelFormValues {
	switch (action.type) {
		case 'textChanged': {
			const changed = { ...values, [action.field]: action.value };
			if (action.field !== 'listenPort' || !values.isTargetPortLinked) {
				return changed;
			}
			return { ...changed, targetPort: action.value };
		}
		case 'kindChanged':
			return { ...values, kind: action.kind };
		case 'openSchemeChanged':
			return { ...values, openScheme: action.scheme };
		case 'openHostChanged':
			return { ...values, openHost: action.host };
		case 'targetPortChanged':
			return {
				...values,
				targetPort: action.value,
				isTargetPortLinked: false,
			};
		case 'targetPortRelinked':
			return {
				...values,
				targetPort: values.listenPort,
				isTargetPortLinked: true,
			};
	}
}
