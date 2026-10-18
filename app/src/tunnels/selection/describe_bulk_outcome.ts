import type { ToastTone } from '#/shared/toasts/toast_store';
import { formatTunnelCount } from '#/tunnels/selection/describe_selection_count';
import type {
	BulkFailure,
	BulkOutcome,
} from '#/tunnels/selection/run_bulk_operation';

/** The past participle of a bulk action, agreed with one tunnel or with several. */
export type BulkParticiple = { singular: string; plural: string };

type BulkReport = { tone: ToastTone; message: string };

const SINGLE_TUNNEL_COUNT = 1;

function agree(count: number, participle: Readonly<BulkParticiple>): string {
	return count > SINGLE_TUNNEL_COUNT ? participle.plural : participle.singular;
}

function describeFailures(failures: readonly BulkFailure[]): string {
	return failures.map(({ name, message }) => `${name} (${message})`).join(', ');
}

/** « 3 tunnels démarrés », or « 2 sur 3 démarrés ; échec : Vite (message) » in the error tone. */
export function describeBulkOutcome(
	{ total, succeededIds, failures }: Readonly<BulkOutcome>,
	participle: Readonly<BulkParticiple>
): BulkReport {
	if (failures.length === 0) {
		return {
			tone: 'success',
			message: `${formatTunnelCount(total)} ${agree(total, participle)}`,
		};
	}
	return {
		tone: 'error',
		message: `${succeededIds.length} sur ${total} ${agree(total, participle)} ; échec : ${describeFailures(failures)}`,
	};
}
