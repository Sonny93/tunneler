use russh::MethodSet;

pub(crate) enum MethodOutcome {
    Authenticated,
    Rejected {
        remaining_methods: MethodSet,
        partial_success: bool,
    },
    NotAttempted,
}
