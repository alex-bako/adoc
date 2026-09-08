//! Local AgentDoc workflow services shared by CLI and protocol adapters.

mod config;
mod context;
mod error;
mod path_policy;
mod use_cases;

pub use config::{ConfigOutputs, EmbeddingsProvider, ProjectConfig};
pub use context::LocalContext;
pub use error::LocalError;
pub use path_policy::{PathPolicy, ProjectRootPathPolicy, UnrestrictedPathPolicy};
pub use use_cases::{
    AssessmentInput, AssessmentOutcome, BuildInput, BuildOutcome, BuildOutputs, CheckInput,
    CheckOutcome, CheckReceiptInput, CheckReceiptOutcome, ContradictionsInput,
    ContradictionsOutcome, DiffInput, DiffOutcome, GraphInput, GraphOutcome, ImpactedChangedSet,
    ImpactedInput, ImpactedOutcome, InitOutcome, MigrateInput, MigrateOutcome, PatchApplyInput,
    PatchApplyOutcome, PatchApplySource, PatchCheckInput, PatchCheckOutcome,
    ProjectArtifactLoadStatus, ProjectArtifactStatus, ProjectStatusArtifacts, ProjectStatusConfig,
    ProjectStatusInput, ProjectStatusOutcome, ProjectStatusPaths, ProjectStatusReadiness,
    ProjectStatusRefresh, ProjectStatusRefreshReport, RepositoryBaselineInput,
    RepositoryBaselineOutcome, ResolvedRetrievalRecord, ResolvedSearchEntry, ReviewInput,
    ReviewOutcome, ReviewPatchSource, SearchInput, SearchOutcome, StaleInput, StaleOutcome,
    WhyInput, WhyOutcome,
};

pub use use_cases::prepare_migration;
