use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::presentation::{CheckStyle, ColorChoice, FormatChoice};

const ROOT_LONG_HELP: &str = "\
Examples:
  adoc init
  adoc check docs
  adoc build docs --out dist
  adoc why billing.refunds.issue-credit
  adoc graph billing.refunds.issue-credit
  adoc patch --check patch.json
  adoc diff main
  adoc search \"refund policy\"
  adoc stale --within 30d
  adoc contradictions --all
  adoc impacted-by --ref main
";
const INIT_LONG_HELP: &str = "\
Examples:
  adoc init
";
const CHECK_LONG_HELP: &str = "\
Examples:
  adoc check
  adoc check docs
  adoc check docs/refunds.adoc
  adoc check --receipt receipt.json --as-of 2026-01-01 \\
    --runtime-binary-digest sha256:<64 hex>
  adoc check --receipt receipt.json --as-of 2026-01-01 \\
    --runtime-binary-digest sha256:<64 hex> \\
    --context-artifact dist/docs.graph.json \\
    --semantic-subject-revision git=head-sha \\
    --semantic-source-revision git=head-sha \\
    --semantic-base-revision git=base-sha \\
    --semantic-head-revision git=head-sha \\
    --semantic-assessment-digest sha256:<64 hex> \\
    --semantic-selection-algorithm changed-only \\
    --semantic-selection-version 1 \\
    --semantic-context-class $SEMANTIC_CONTEXT_CLASS_JSON \\
    --semantic-authorized-scope repo:billing \\
    --semantic-object-context $SEMANTIC_OBJECT_CONTEXT_JSON \\
    --semantic-capability-policy $SEMANTIC_POLICY_JSON \\
    --semantic-context semantic-context.json

--receipt runs the same validation and writes a digest-bound
adoc.validation_receipt.v0 envelope, or adoc.validation_receipt.v1 when
--source-invocation is supplied (SEMANTICS S6). Receipts are deterministic —
no wall-clock timestamps — so --receipt requires an explicit --as-of and the
invoking harness's attested
--runtime-binary-digest (a binary cannot hash itself; the harness
verifies its pin before invoking, see scripts/validation-runtime/).
When --semantic-context is supplied, receipt mode validates its exact
revision, digest, completeness, authorized scope, and closed citations.
The revision, assessment, selection, context-class, authorized-scope, object-
context, and capability-policy flags are trusted expectations; repeat scope
and object-context flags as needed. Scope may be omitted when none is
authorized. Trusted expectations are never inferred from context.
Object-context may be omitted for citation-free contexts; any included
citation without a trusted mapping fails closed.
Graph-backed contexts require the exact --context-artifact. The local CLI
has no managed-revision store, so managed-revision contexts fail closed.
Diff-hunk and Source Assertion citations require E4.1 Source Record
projections, which local receipt mode does not yet have, so they fail closed.
Local graph projections permit no truncated content variants, so truncated
items also fail closed here; domain callers may supply trusted variants.
";
const MIGRATE_LONG_HELP: &str = "\
Default is a dry run: prints what would be migrated plus the migrate.*
diagnostics, and writes nothing. --write creates <name>.adoc beside each
source and removes the source .md — leaving both would compile duplicate
page IDs. --write refuses (all-or-nothing, nothing written or removed) when
any source .md is not committed-and-clean in git; --force overrides. A
committed source plus `git` is what makes the removal reversible.

--export runs the reverse: strict prose-mode .adoc back to Markdown, with
the same dry-run default and --write/--force semantics (writes <name>.md,
removes the source .adoc). A page containing typed blocks refuses the whole
run with migrate.export_typed_blocks_present — exporting typed knowledge to
Markdown is lossy by definition.

Exit codes: 0 converted (or dry-run clean); 1 refused or migration errors;
2 usage errors. Warnings never fail the run.

Examples:
  adoc migrate
  adoc migrate docs
  adoc migrate docs --write
  adoc migrate docs --write --force
  adoc migrate docs --export
  adoc migrate docs --export --write
";
const BUILD_LONG_HELP: &str = "\
Examples:
  adoc build
  adoc build docs --out dist
  adoc build docs --out dist --no-embeddings
";
const WHY_LONG_HELP: &str = "\
Examples:
  adoc why billing.refunds.issue-credit
  adoc why billing.refunds.issue-credit --artifact dist/docs.graph.json
  adoc why billing.refunds.issue-credit --format json
";
const GRAPH_LONG_HELP: &str = "\
Examples:
  adoc graph billing.refunds.issue-credit
  adoc graph billing.refunds.issue-credit --direction outgoing
  adoc graph billing.refunds.issue-credit --relation depends_on --format json
";
const PATCH_LONG_HELP: &str = "\
--check validates without writing; --apply validates, then rewrites exactly
the targeted source spans (formatting-preserving), re-checks, and reports.
Apply writes to the working tree only and never auto-reverts: review the
result with git diff, undo with git checkout. After a successful apply the
graph artifact is stale — run `adoc build`.

Apply exit codes: 0 applied and post-check clean; 1 refused, nothing written;
2 applied but the post-check found new errors (stop and review).

Examples:
  adoc patch --check patch.json
  adoc patch --check patch.json --artifact dist/docs.graph.json
  adoc patch --check patch.json --format json
  adoc patch --apply patch.json
  cat patch.json | adoc patch --apply @- --format json
";
const DIFF_LONG_HELP: &str = "\
Examples:
  adoc diff main
  adoc diff main --format json
  adoc diff main --format markdown
  adoc diff HEAD~1
";
const REVIEW_LONG_HELP: &str = "\
Examples:
  adoc review main
  adoc review main --format json
  adoc review main --format markdown
  adoc review HEAD~1
";
const SEARCH_LONG_HELP: &str = "\
Search returns one blended, RRF-ranked list of Knowledge Object and prose
records (adoc.retrieval.v1). Object ID pins stay on top and ride above the
--top budget (--top bounds scored hits; pinned ids are always included);
prose records are orientation context, never citable knowledge. Setting any Knowledge Object
metadata filter (--kind, --status, --owner, --source-path, --related-to)
implies object intent and suppresses prose records.

Examples:
  adoc search \"refund policy\"
  adoc search \"refund policy\" --kind claim --top 5
  adoc search \"refund policy\" --related-to billing.refunds.issue-credit --relation depends_on
  adoc search billing.refunds --lexical
  adoc search \"getting started\" --prose-only
  adoc search \"refund policy\" --objects-only --format json
";
const STALE_LONG_HELP: &str = "\
Staleness and review-overdue-ness are re-derived from authored fields at the
time of the query, not read from the artifact's build-time projection. The
command is a query, not a gate: it exits 0 whether or not records exist.

Examples:
  adoc stale
  adoc stale --within 30d
  adoc stale --within 30d --format json
  adoc stale --artifact dist/docs.graph.json
";
const CONTRADICTIONS_LONG_HELP: &str = "\
Lists every unresolved contradiction plus every contradicted claim — implicated
by an unresolved contradiction or authored as contradicted — with the
contradiction ids that implicate it, so consumers never join the two lists.
The output is a pure function of the graph artifact (no clock). The command is
a query, not a gate: it exits 0 whether or not findings exist.

Examples:
  adoc contradictions
  adoc contradictions --all
  adoc contradictions --format json
  adoc contradictions --artifact dist/docs.graph.json
";

const IMPACTED_BY_LONG_HELP: &str = "\
Answers \"this code changed — which knowledge is now suspect?\" over the graph
artifact: verified claims and accepted decisions whose declared impacts: paths
or evidence paths (inline source_code/test values, or the path of a referenced
source object) exactly match a changed file. No recompile, no globs. The
command is a query, not a gate: it exits 0 whether or not anything is
impacted.

Exactly one input shape: explicit changed paths, or --ref <git-ref> to derive
the changed set from git (the base ref against the working tree, the same
shape as `adoc review <ref>`).

On input or environment errors (exit 1/2), --format json still emits a valid
envelope with the diagnostics; --format markdown writes a blockquote error
to stdout; plain/styled write fix-oriented messages to stderr only. Use
--format json for unattended runs.

Examples:
  adoc impacted-by crates/billing/src/refund.rs
  adoc impacted-by src/a.rs src/b.rs --format json
  adoc impacted-by --ref main
  adoc impacted-by --ref main --format markdown
";
const ASSESS_CHANGES_LONG_HELP: &str = "\
Produces the deterministic adoc.change_assessment.v0 envelope for one Git
comparison. The comparison-base configuration is effective; head policy is
reported as prospective. Complete outcomes are advisory and exit 0. Partial,
invalid, or not-evaluated assessments exit 2.

Examples:
  adoc assess-changes --base main
  adoc assess-changes --base main --head HEAD --as-of 2026-07-22 --format json
  adoc assess-changes --base main --format markdown
";
const BASELINE_LONG_HELP: &str = "\
Inventories every tracked path at one immutable Git ref against AgentDoc
knowledge. Readiness requires valid source and authoritative coverage for
every non-excluded path.

Examples:
  adoc baseline --ref HEAD
  adoc baseline --ref main --as-of 2026-07-28 --format json
";

/// Parse the `--within <N>d` horizon grammar (same `[0-9]+d` shape as the
/// `review_interval:` field) into a day count.
fn parse_within_days(value: &str) -> Result<u32, String> {
    let error = || format!("expected a day count like `30d`, got `{value}`");
    let days = value.strip_suffix('d').ok_or_else(error)?;
    if days.is_empty() || !days.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(error());
    }
    days.parse::<u32>().map_err(|_| error())
}

fn parse_evaluation_date(value: &str) -> Result<chrono::NaiveDate, String> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| format!("expected a date like `2026-07-22`, got `{value}`"))
}

fn parse_exact_revision(value: &str) -> Result<adoc_core::ExactRevision, String> {
    let error = || {
        format!(
            "expected an exact revision like `git=head-sha` or JSON with system/value, got `{value}`"
        )
    };
    let revision = if value.starts_with('{') {
        serde_json::from_str(value).map_err(|_| error())?
    } else {
        if value.bytes().filter(|byte| *byte == b'=').count() != 1 {
            return Err(error());
        }
        let (system, value) = value.split_once('=').ok_or_else(error)?;
        adoc_core::ExactRevision {
            system: system.to_string(),
            value: value.to_string(),
        }
    };
    if !adoc_core::is_semantic_context_text(&revision.system)
        || !adoc_core::is_semantic_context_text(&revision.value)
    {
        return Err(error());
    }
    Ok(revision)
}

fn parse_boxed_exact_revision(value: &str) -> Result<Box<adoc_core::ExactRevision>, String> {
    parse_exact_revision(value).map(Box::new)
}

fn parse_sha256_digest(value: &str) -> Result<String, String> {
    if adoc_core::is_sha256_digest(value) {
        return Ok(value.to_string());
    }
    Err(format!(
        "expected a lowercase sha256 digest like `sha256:<64 hex>`, got `{value}`"
    ))
}

fn parse_semantic_text(value: &str) -> Result<String, String> {
    adoc_core::is_semantic_context_text(value)
        .then(|| value.to_string())
        .ok_or_else(|| format!("expected non-blank semantic text, got `{value}`"))
}

fn parse_capability_policy(value: &str) -> Result<Box<adoc_core::CapabilityPolicy>, String> {
    let mut policy: adoc_core::CapabilityPolicy = serde_json::from_str(value)
        .map_err(|_| "expected a complete capability-policy JSON object".to_string())?;
    policy.rules.sort_by_key(|rule| rule.reason);
    adoc_core::is_valid_capability_policy(&policy)
        .then(|| Box::new(policy))
        .ok_or_else(|| "expected one capability-policy rule for every closed reason".to_string())
}

fn parse_graph_object_context(
    value: &str,
) -> Result<adoc_core::GraphObjectContextExpectation, String> {
    let context: adoc_core::GraphObjectContextExpectation =
        serde_json::from_str(value).map_err(|_| {
            "expected graph-object context JSON with object_id/class_id/scope_ref".to_string()
        })?;
    let valid = [
        context.object_id.as_str(),
        context.class_id.as_str(),
        context.scope_ref.as_str(),
    ]
    .into_iter()
    .all(adoc_core::is_semantic_context_text);
    valid
        .then_some(context)
        .ok_or_else(|| "graph-object context fields must be non-blank semantic text".to_string())
}

fn parse_context_class(value: &str) -> Result<adoc_core::ContextClass, String> {
    let class: adoc_core::ContextClass = serde_json::from_str(value).map_err(|_| {
        "expected context-class JSON with class_id/requirement/byte_budget".to_string()
    })?;
    (adoc_core::is_semantic_context_text(&class.class_id) && class.byte_budget > 0)
        .then_some(class)
        .ok_or_else(|| {
            "context class ID must be non-blank and byte budget must be positive".to_string()
        })
}

/// The output format requested on the command line (`--format`).
#[derive(Clone, Copy, Default, ValueEnum)]
pub(crate) enum CliFormat {
    /// Auto-detect: styled when stdout is a TTY, plain otherwise.
    #[default]
    Auto,
    /// Plain uncoloured text.
    Plain,
    /// Styled text with ANSI colour codes.
    Styled,
    /// Machine-readable JSON.
    Json,
    /// PR-comment-ready GitHub-flavored Markdown (only supported by
    /// `adoc check`, `adoc diff`, `adoc review`, and `adoc impacted-by`).
    Markdown,
}

impl From<CliFormat> for FormatChoice {
    fn from(f: CliFormat) -> Self {
        match f {
            CliFormat::Auto => Self::Auto,
            CliFormat::Plain => Self::Plain,
            CliFormat::Styled => Self::Styled,
            CliFormat::Json => Self::Json,
            CliFormat::Markdown => Self::Markdown,
        }
    }
}

/// The Markdown layout requested on the command line (`adoc check --style`).
/// Named "layout" in help text to keep it apart from `--format styled` and
/// ANSI styling.
#[derive(Clone, Copy, Default, ValueEnum)]
pub(crate) enum CliCheckStyle {
    /// One bullet per diagnostic; remediation help collapsed.
    #[default]
    Compact,
    /// One table row per diagnostic; remediation help collapsed.
    Table,
    /// Per-file grouping with object_id/help sub-bullets.
    Detailed,
}

impl From<CliCheckStyle> for CheckStyle {
    fn from(s: CliCheckStyle) -> Self {
        match s {
            CliCheckStyle::Compact => Self::Compact,
            CliCheckStyle::Table => Self::Table,
            CliCheckStyle::Detailed => Self::Detailed,
        }
    }
}

/// The colour mode requested on the command line (`--color`).
#[derive(Clone, Copy, Default, ValueEnum)]
pub(crate) enum CliColor {
    /// Enable colour only when stdout is a TTY and `NO_COLOR` is unset.
    #[default]
    Auto,
    /// Always emit ANSI colour codes.
    Always,
    /// Never emit ANSI colour codes.
    Never,
}

impl From<CliColor> for ColorChoice {
    fn from(c: CliColor) -> Self {
        match c {
            CliColor::Auto => Self::Auto,
            CliColor::Always => Self::Always,
            CliColor::Never => Self::Never,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum CliGraphRelation {
    #[value(name = "depends_on")]
    DependsOn,
    Supersedes,
    #[value(name = "related_to")]
    RelatedTo,
}

impl From<CliGraphRelation> for adoc_core::GraphRelationKind {
    fn from(value: CliGraphRelation) -> Self {
        match value {
            CliGraphRelation::DependsOn => Self::DependsOn,
            CliGraphRelation::Supersedes => Self::Supersedes,
            CliGraphRelation::RelatedTo => Self::RelatedTo,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum CliGraphDirection {
    Outgoing,
    Incoming,
    Both,
}

impl From<CliGraphDirection> for adoc_core::GraphDirection {
    fn from(value: CliGraphDirection) -> Self {
        match value {
            CliGraphDirection::Outgoing => Self::Outgoing,
            CliGraphDirection::Incoming => Self::Incoming,
            CliGraphDirection::Both => Self::Both,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "adoc",
    version,
    about = "AgentDoc Local CLI for checking, building, and querying AgentDoc Source.",
    after_long_help = ROOT_LONG_HELP
)]
pub(crate) struct Cli {
    /// Output format.  `auto` selects `styled` when stdout is a TTY and
    /// `NO_COLOR` is unset, otherwise `plain`.
    #[arg(long, global = true, value_enum, default_value = "auto")]
    pub(crate) format: CliFormat,

    /// Colour output.  `auto` enables colour only on a TTY without `NO_COLOR`.
    /// `always` overrides the TTY check.  `never` disables colour.
    #[arg(long, global = true, value_enum, default_value = "auto")]
    pub(crate) color: CliColor,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Export exact-snapshot source evidence and inactive candidate inputs.
    MigrationImport {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        job: PathBuf,
        #[arg(long)]
        repository: PathBuf,
        #[arg(long)]
        runtime_binary_digest: String,
    },
    /// Prepare a full exact-commit migration receipt in an isolated worker.
    MigrationPrepare {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        repository: PathBuf,
        #[arg(long)]
        runtime_binary_digest: String,
    },
    /// Project an authorized retained corpus; bounded JSON on stdin and stdout.
    PortableProject,
    /// Runtime port for an explicitly authorized managed corpus; always emits retrieval JSON.
    ManagedRetrieve {
        /// Require this runtime to label every returned internal/restricted Knowledge Object.
        #[arg(long)]
        require_sensitive_classification: bool,
        /// Require partial field projection and source access attribution.
        #[arg(long)]
        require_field_projection: bool,
        /// Refuse older runtimes before sending governed field-lowering metadata.
        #[arg(long)]
        require_field_declassification: bool,
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        /// Create a private contributor manifest for the trusted caller's final authorization check.
        #[arg(long, value_name = "PATH")]
        manifest_out: Option<PathBuf>,
        #[command(subcommand)]
        operation: ManagedRetrievalCommand,
    },
    #[command(
        about = "Create AgentDoc config and starter docs.",
        after_long_help = INIT_LONG_HELP
    )]
    Init,
    #[command(
        about = "Check AgentDoc Source for strict-mode diagnostics.",
        after_long_help = CHECK_LONG_HELP
    )]
    Check {
        /// AgentDoc Source file or directory to check.
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        /// Markdown layout for `--format markdown`; ignored by other formats.
        #[arg(long, value_enum, default_value = "compact")]
        style: CliCheckStyle,
        /// Pin lifecycle evaluation to this UTC calendar date.
        #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_evaluation_date)]
        as_of: Option<chrono::NaiveDate>,
        /// Write a digest-bound receipt: v0 normally, v1 with `--source-invocation`.
        /// Receipt mode prints plain diagnostics only: an explicit
        /// `--style` conflicts, and `--format markdown` is refused at
        /// dispatch — never silently ignored.
        #[arg(
            long,
            value_name = "PATH",
            requires = "as_of",
            requires = "runtime_binary_digest",
            conflicts_with = "style"
        )]
        receipt: Option<PathBuf>,
        /// Harness-attested sha256 of the invoking adoc binary
        /// (`sha256:<64 hex>`); recorded verbatim in the receipt.
        #[arg(long, value_name = "DIGEST", requires = "receipt")]
        runtime_binary_digest: Option<String>,
        /// Source invocation manifest to digest-bind into the receipt.
        #[arg(long, value_name = "PATH", requires = "receipt")]
        source_invocation: Option<PathBuf>,
        /// Graph artifact to validate against the recompiled source
        /// (exact-match adoc.graph.v6; drift fails the receipt).
        #[arg(long, value_name = "PATH", requires = "receipt")]
        context_artifact: Option<PathBuf>,
        /// Digest-bound adoc.semantic_context.v0 validated against --as-of
        /// and, when graph-backed, the same --context-artifact bytes.
        #[arg(
            long,
            value_name = "PATH",
            requires = "receipt",
            requires_all = [
                "semantic_subject_revision",
                "semantic_source_revision",
                "semantic_base_revision",
                "semantic_head_revision",
                "semantic_assessment_digest",
                "semantic_selection_algorithm",
                "semantic_selection_version",
                "semantic_context_class",
                "semantic_capability_policy"
            ]
        )]
        semantic_context: Option<PathBuf>,
        /// Trusted subject revision expected in --semantic-context.
        #[arg(long, value_name = "SYSTEM=VALUE", requires = "semantic_context", value_parser = parse_exact_revision)]
        semantic_subject_revision: Option<adoc_core::ExactRevision>,
        /// Trusted source revision expected in --semantic-context.
        #[arg(long, value_name = "SYSTEM=VALUE", requires = "semantic_context", value_parser = parse_boxed_exact_revision)]
        semantic_source_revision: Option<Box<adoc_core::ExactRevision>>,
        /// Trusted base revision expected in --semantic-context.
        // Boxed revision payloads keep `Commands` under clippy::large_enum_variant.
        #[arg(long, value_name = "SYSTEM=VALUE", requires = "semantic_context", value_parser = parse_boxed_exact_revision)]
        semantic_base_revision: Option<Box<adoc_core::ExactRevision>>,
        /// Trusted head revision expected in --semantic-context.
        #[arg(long, value_name = "SYSTEM=VALUE", requires = "semantic_context", value_parser = parse_boxed_exact_revision)]
        semantic_head_revision: Option<Box<adoc_core::ExactRevision>>,
        /// Trusted assessment digest expected in --semantic-context.
        #[arg(long, value_name = "DIGEST", requires = "semantic_context", value_parser = parse_sha256_digest)]
        semantic_assessment_digest: Option<String>,
        /// Trusted selection algorithm expected in --semantic-context.
        #[arg(long, value_name = "ALGORITHM", requires = "semantic_context", value_parser = parse_semantic_text)]
        semantic_selection_algorithm: Option<String>,
        /// Trusted selection version expected in --semantic-context.
        #[arg(long, value_name = "VERSION", requires = "semantic_context", value_parser = parse_semantic_text)]
        semantic_selection_version: Option<String>,
        /// Trusted complete context class as JSON; repeat for every required or optional class.
        #[arg(long, value_name = "JSON", requires = "semantic_context", value_parser = parse_context_class)]
        semantic_context_class: Vec<adoc_core::ContextClass>,
        /// Trusted authorized scope; repeat for each scope.
        #[arg(long, value_name = "SCOPE", requires = "semantic_context", value_parser = parse_semantic_text)]
        semantic_authorized_scope: Vec<String>,
        /// Trusted Object ID to class/scope mapping as JSON; repeat per cited object.
        #[arg(long, value_name = "JSON", requires = "semantic_context", value_parser = parse_graph_object_context)]
        semantic_object_context: Vec<adoc_core::GraphObjectContextExpectation>,
        /// Trusted complete capability policy as JSON.
        // Boxed for the same `Commands` enum-size reason as the base and head revision payloads.
        #[arg(long, value_name = "JSON", requires = "semantic_context", value_parser = parse_capability_policy)]
        semantic_capability_policy: Option<Box<adoc_core::CapabilityPolicy>>,
    },
    #[command(
        about = "Convert Markdown sources to prose-mode .adoc, or back with --export (dry-run by default).",
        after_long_help = MIGRATE_LONG_HELP
    )]
    Migrate {
        /// Source file or directory to convert.
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        /// Write each source's target file and remove the source files.
        #[arg(long)]
        write: bool,
        /// Skip the committed-and-clean git refusal for --write.
        #[arg(long)]
        force: bool,
        /// Export prose-mode .adoc sources back to Markdown instead of importing.
        #[arg(long)]
        export: bool,
    },
    #[command(
        about = "Build HTML, graph, and search artifacts.",
        after_long_help = BUILD_LONG_HELP
    )]
    Build {
        /// AgentDoc Source file or directory to build.
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        /// Output directory for docs.html, docs.graph.json, and docs.search.json.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Skip embedding generation and search artifact writes.
        #[arg(long)]
        no_embeddings: bool,
        /// Pin lifecycle evaluation to this UTC calendar date.
        #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_evaluation_date)]
        as_of: Option<chrono::NaiveDate>,
        /// HTML visibility ceiling: public, internal, or restricted.
        #[arg(long, value_name = "AUDIENCE")]
        audience: Option<String>,
    },
    #[command(
        about = "Explain one Knowledge Object from a compiled artifact.",
        after_long_help = WHY_LONG_HELP
    )]
    Why {
        /// Object ID to explain.
        #[arg(value_name = "OBJECT_ID")]
        object_id: String,
        #[arg(
            long,
            help = "Graph JSON artifact path (default: config outputs.graph, then dist/docs.graph.json)"
        )]
        artifact: Option<PathBuf>,
    },
    #[command(
        about = "Traverse Knowledge Object relations from graph artifacts.",
        after_long_help = GRAPH_LONG_HELP
    )]
    Graph {
        /// Object ID to use as the graph traversal root.
        #[arg(value_name = "OBJECT_ID")]
        object_id: String,
        #[arg(
            long,
            help = "Graph JSON artifact path (default: config outputs.graph, then dist/docs.graph.json)"
        )]
        artifact: Option<PathBuf>,
        #[arg(long, value_enum)]
        relation: Option<CliGraphRelation>,
        #[arg(long, value_enum)]
        direction: Option<CliGraphDirection>,
    },
    #[command(
        about = "List stale, review-overdue, and expiring Knowledge Objects from graph artifacts.",
        after_long_help = STALE_LONG_HELP
    )]
    Stale {
        #[arg(
            long,
            help = "Graph JSON artifact path (default: config outputs.graph, then dist/docs.graph.json)"
        )]
        artifact: Option<PathBuf>,
        /// Additionally list verified objects expiring within the next N days,
        /// e.g. `--within 30d`.
        #[arg(long, value_name = "Nd", value_parser = parse_within_days)]
        within: Option<u32>,
    },
    #[command(
        about = "List unresolved contradictions and contradicted claims from graph artifacts.",
        after_long_help = CONTRADICTIONS_LONG_HELP
    )]
    Contradictions {
        #[arg(
            long,
            help = "Graph JSON artifact path (default: config outputs.graph, then dist/docs.graph.json)"
        )]
        artifact: Option<PathBuf>,
        /// Include resolved and dismissed contradictions in the listing.
        #[arg(long)]
        all: bool,
    },
    #[command(
        name = "impacted-by",
        about = "List Knowledge Objects implicated by changed source paths.",
        after_long_help = IMPACTED_BY_LONG_HELP
    )]
    ImpactedBy {
        /// Changed repo-relative file paths (as emitted by
        /// `git diff --name-only`). Mutually exclusive with `--ref`.
        #[arg(
            value_name = "PATH",
            required_unless_present = "git_ref",
            conflicts_with = "git_ref"
        )]
        paths: Vec<String>,
        /// Derive the changed set from git: the base ref against the working
        /// tree (the `adoc review <ref>` shape).
        #[arg(long = "ref", value_name = "GIT_REF")]
        git_ref: Option<String>,
        #[arg(
            long,
            help = "Graph JSON artifact path (default: config outputs.graph, then dist/docs.graph.json)"
        )]
        artifact: Option<PathBuf>,
    },
    #[command(
        name = "assess-changes",
        about = "Assess one Git change set against AgentDoc knowledge.",
        after_long_help = ASSESS_CHANGES_LONG_HELP
    )]
    AssessChanges {
        /// Requested base ref; the unique merge base with head is assessed.
        #[arg(long, value_name = "GIT_REF")]
        base: String,
        /// Immutable head ref. Omit to assess the current worktree.
        #[arg(long, value_name = "GIT_REF")]
        head: Option<String>,
        /// Pin lifecycle evaluation to this UTC calendar date.
        #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_evaluation_date)]
        as_of: Option<chrono::NaiveDate>,
    },
    #[command(
        about = "Inventory repository-wide AgentDoc coverage at one Git ref.",
        after_long_help = BASELINE_LONG_HELP
    )]
    Baseline {
        /// Immutable repository ref to inventory.
        #[arg(long = "ref", value_name = "GIT_REF")]
        git_ref: String,
        /// Pin lifecycle evaluation to this UTC calendar date.
        #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_evaluation_date)]
        as_of: Option<chrono::NaiveDate>,
    },
    #[command(
        name = "semantic-context",
        about = "Build canonical adoc.semantic_context.v0 from portable producer input."
    )]
    SemanticContext {
        #[arg(long, value_name = "INPUT_JSON")]
        input: PathBuf,
        #[arg(long, value_name = "CONTEXT_JSON")]
        out: PathBuf,
    },
    #[command(
        name = "proposal-record",
        about = "Build the canonical adoc.proposal.v0 record from exact patch files and their bindings."
    )]
    ProposalRecord {
        /// Producer input: bindings, patch files, and optional supersedes/dispositions.
        #[arg(long, value_name = "INPUT_JSON")]
        input: PathBuf,
        #[arg(long, value_name = "RECORD_JSON")]
        out: PathBuf,
    },
    #[command(
        name = "semantic-executor",
        about = "Validate one semantic executor response and write its deterministic receipt."
    )]
    SemanticExecutor {
        #[arg(long, value_name = "REQUEST_JSON")]
        request: PathBuf,
        #[arg(long, value_name = "ASSESSMENT_JSON")]
        assessment: PathBuf,
        /// Record an adapter/process failure without accepting assessment bytes.
        #[arg(long, value_name = "FAILURE_CODE", value_parser = parse_semantic_text)]
        failure_code: Option<String>,
        #[arg(long, value_name = "RECEIPT_JSON")]
        receipt: PathBuf,
        #[arg(long, value_name = "VALIDATED_ASSESSMENT_JSON")]
        validated_assessment: PathBuf,
        /// Write the exact compact validated request bytes bound by the receipt digest.
        #[arg(long, value_name = "VALIDATED_REQUEST_JSON")]
        validated_request: Option<PathBuf>,
        /// Authenticated reviewing Principal supplied by the trusted caller, not the request file.
        #[arg(long, requires = "requesting_principal_id", value_parser = parse_semantic_text)]
        reviewing_principal_id: Option<String>,
        /// Authenticated requesting Principal supplied by the trusted caller, not the request file.
        #[arg(long, requires = "reviewing_principal_id", value_parser = parse_semantic_text)]
        requesting_principal_id: Option<String>,
    },
    #[command(
        about = "Validate one AgentDoc patch document against graph artifacts, or apply it to source.",
        after_long_help = PATCH_LONG_HELP
    )]
    Patch {
        /// Patch JSON document to validate (read-only).
        #[arg(
            long,
            value_name = "PATCH_JSON",
            conflicts_with = "apply",
            required_unless_present = "apply"
        )]
        check: Option<PathBuf>,
        /// Patch JSON document to validate and apply to AgentDoc source.
        /// Pass a path, or `@-` to read the document from stdin.
        #[arg(long, value_name = "PATCH_JSON_OR_@-")]
        apply: Option<String>,
        #[arg(
            long,
            help = "Graph JSON artifact path (default: config outputs.graph, then dist/docs.graph.json)"
        )]
        artifact: Option<PathBuf>,
        /// Pin lifecycle evaluation to this UTC calendar date.
        #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_evaluation_date)]
        as_of: Option<chrono::NaiveDate>,
    },
    #[command(
        about = "Diff Knowledge Objects between a git ref and the working tree.",
        after_long_help = DIFF_LONG_HELP
    )]
    Diff {
        /// Base git ref to diff against. The current working tree is the head.
        #[arg(value_name = "BASE_REF")]
        base_ref: String,
    },
    #[command(
        about = "Review Knowledge Object changes with source-path impact and required reviewers.",
        after_long_help = REVIEW_LONG_HELP
    )]
    Review {
        /// Base git ref to review against. The current working tree is the head.
        #[arg(value_name = "BASE_REF")]
        base_ref: String,
        /// Optional adoc.patch.v0 JSON file to validate against the head graph.
        /// When supplied, the review envelope embeds an adoc.patch.check.v0
        /// report and unions patch-driven proof obligations into the
        /// top-level obligation list.
        #[arg(long, value_name = "PATCH_JSON")]
        patch: Option<PathBuf>,
    },
    #[command(
        about = "Search compiled Knowledge Objects.",
        after_long_help = SEARCH_LONG_HELP
    )]
    Search {
        /// Query text or Object ID prefix to search for.
        #[arg(value_name = "QUERY")]
        query: String,
        #[arg(
            long,
            help = "Graph JSON artifact path (default: config outputs.graph, then dist/docs.graph.json)"
        )]
        artifact: Option<PathBuf>,
        #[arg(
            long,
            help = "Search artifact path (default: config outputs.search, then dist/docs.search.json)"
        )]
        search_artifact: Option<PathBuf>,
        #[arg(long, conflicts_with = "lexical")]
        semantic: bool,
        /// Force deterministic BM25 + Object ID ranking, skipping vectors.
        /// Hybrid fusion is the default when neither --semantic nor
        /// --lexical is set.
        #[arg(long, conflicts_with = "semantic")]
        lexical: bool,
        /// Return only Knowledge Object records (the pre-V1.7 result set).
        #[arg(long, conflicts_with = "prose_only")]
        objects_only: bool,
        /// Return only prose records. Prose has no Knowledge Object metadata,
        /// so this conflicts with the metadata filters. Semantic prose search
        /// works since V1.7.2 (adoc.search.v2 prose vectors).
        #[arg(
            long,
            conflicts_with_all = [
                "objects_only", "kind", "status", "owner",
                "source_path", "related_to",
            ]
        )]
        prose_only: bool,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        source_path: Option<String>,
        #[arg(long)]
        related_to: Option<String>,
        #[arg(long, value_enum, requires = "related_to")]
        relation: Option<CliGraphRelation>,
        #[arg(long, value_enum, requires = "related_to")]
        direction: Option<CliGraphDirection>,
        #[arg(long, default_value = "10")]
        top: NonZeroUsize,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ManagedRetrievalCommand {
    Search {
        query: String,
        #[arg(long, value_enum, default_value = "lexical")]
        mode: ManagedSearchMode,
        #[arg(long, default_value = "20")]
        top: NonZeroUsize,
    },
    Why {
        object_id: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ManagedSearchMode {
    Lexical,
    Semantic,
    Hybrid,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, parse_context_class, parse_exact_revision, parse_sha256_digest};

    #[test]
    fn semantic_binding_parsers_reject_values_no_valid_context_can_match() {
        assert!(parse_exact_revision("git=head-sha").is_ok());
        assert!(parse_exact_revision(r#"{"system":"a=b","value":"c"}"#).is_ok());
        for invalid in ["git= head-sha", " git=head-sha", "git=head\nsha", "a=b=c"] {
            assert!(parse_exact_revision(invalid).is_err(), "{invalid:?}");
        }

        let digest = format!("sha256:{}", "a".repeat(64));
        assert_eq!(parse_sha256_digest(&digest), Ok(digest));
        for invalid in ["sha256:ABC", "sha256:abc", "not-a-digest"] {
            assert!(parse_sha256_digest(invalid).is_err(), "{invalid:?}");
        }

        assert!(
            parse_context_class(
                r#"{"class_id":"changed_source","requirement":"required","byte_budget":4096}"#
            )
            .is_ok()
        );
        for invalid in [
            r#"{"class_id":"changed_source","requirement":"required","byte_budget":0}"#,
            r#"{"class_id":"","requirement":"required","byte_budget":4096}"#,
        ] {
            assert!(parse_context_class(invalid).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn citation_free_semantic_context_does_not_require_an_object_projection() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let policy = r#"{"version":"semantic-context-policy-v1","rules":[{"reason":"permission","outcome":"insufficient"},{"reason":"retention","outcome":"insufficient"},{"reason":"source_outage","outcome":"insufficient"},{"reason":"truncation","outcome":"insufficient"},{"reason":"resource_limit","outcome":"insufficient"}]}"#;
        let parsed = Cli::try_parse_from([
            "adoc",
            "check",
            "--receipt",
            "receipt.json",
            "--as-of",
            "2026-01-01",
            "--runtime-binary-digest",
            digest.as_str(),
            "--semantic-context",
            "semantic-context.json",
            "--semantic-subject-revision",
            "git=head-sha",
            "--semantic-source-revision",
            "git=head-sha",
            "--semantic-base-revision",
            "git=base-sha",
            "--semantic-head-revision",
            "git=head-sha",
            "--semantic-assessment-digest",
            digest.as_str(),
            "--semantic-selection-algorithm",
            "changed-only",
            "--semantic-selection-version",
            "1",
            "--semantic-context-class",
            r#"{"class_id":"changed_source","requirement":"required","byte_budget":4096}"#,
            "--semantic-authorized-scope",
            "repo:billing",
            "--semantic-capability-policy",
            policy,
        ]);

        if let Err(error) = parsed {
            panic!("unexpected usage error: {error}");
        }
    }
}
