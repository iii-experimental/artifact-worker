use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, ArtifactError>;

#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    OpenApi,
    Graphql,
    Har,
    Docs,
    Url,
    Manual,
}

impl Default for SourceType {
    fn default() -> Self {
        Self::Manual
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactInput {
    pub name: String,
    pub goal: Option<String>,
    pub source_type: Option<SourceType>,
    pub source: Option<String>,
    #[serde(default)]
    pub functions: Vec<String>,
    pub output_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SideEffects {
    Read,
    Write,
    Sync,
    ExternalCall,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerFunctionPlan {
    pub function_id: String,
    pub purpose: String,
    pub side_effects: SideEffects,
    pub inputs: serde_json::Value,
    pub output: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerPlan {
    pub worker_name: String,
    pub namespace: String,
    pub source_type: SourceType,
    pub source: Option<String>,
    pub goal: String,
    pub functions: Vec<WorkerFunctionPlan>,
    pub uses_workers: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InspectResult {
    pub name: String,
    pub namespace: String,
    pub source_type: SourceType,
    pub source: Option<String>,
    pub suggested_functions: Vec<String>,
    pub recommendation: String,
    pub existing_workers_to_use: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedWorker {
    pub output_dir: PathBuf,
    pub worker_path: PathBuf,
    pub manifest_path: PathBuf,
    pub plan: WorkerPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerificationReport {
    pub ok: bool,
    pub worker_path: PathBuf,
    pub function_count: usize,
    pub missing_registrations: Vec<String>,
}

pub fn inspect_artifact(input: &ArtifactInput) -> InspectResult {
    let namespace = slugify(&input.name);
    let source_type = input.source_type.clone().unwrap_or_default();
    let functions = infer_functions(input);
    InspectResult {
        name: input.name.clone(),
        namespace: namespace.clone(),
        source_type,
        source: input.source.clone(),
        suggested_functions: functions
            .iter()
            .map(|function| format!("{}::{}", namespace, slugify(function)))
            .collect(),
        recommendation:
            "Generate a narrow iii worker around the specific job, not a generic full API wrapper."
                .into(),
        existing_workers_to_use: vec![
            "iii-state".into(),
            "iii-queue".into(),
            "iii-sandbox".into(),
            "iii-observability".into(),
        ],
    }
}

pub fn plan_worker(input: &ArtifactInput) -> WorkerPlan {
    let namespace = slugify(&input.name);
    let source_type = input.source_type.clone().unwrap_or_default();
    let functions = infer_functions(input)
        .iter()
        .map(|function| plan_function(&namespace, function))
        .collect();

    WorkerPlan {
        worker_name: format!("{}-worker", namespace.replace('_', "-")),
        namespace: namespace.clone(),
        source_type,
        source: input.source.clone(),
        goal: input
            .goal
            .clone()
            .unwrap_or_else(|| format!("Expose focused agent-operable functions for {}.", input.name)),
        functions,
        uses_workers: vec![
            "iii-state".into(),
            "iii-queue".into(),
            "iii-sandbox".into(),
            "iii-http".into(),
            "iii-observability".into(),
        ],
        notes: vec![
            "Keep function count small and job-specific.".into(),
            "Prefer read-only functions unless the worker explicitly syncs or mutates external state.".into(),
            "Persist manifests and source fingerprints through iii-state.".into(),
            "Run generated code checks inside iii-sandbox before publishing.".into(),
        ],
    }
}

pub fn generate_worker(input: &ArtifactInput) -> Result<GeneratedWorker> {
    let plan = plan_worker(input);
    let output_dir = input
        .output_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("generated").join(&plan.worker_name));
    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let manifest_path = output_dir.join("artifact.manifest.json");
    let worker_path = src_dir.join("main.rs");

    fs::write(&manifest_path, serde_json::to_string_pretty(&plan)? + "\n")?;
    fs::write(&worker_path, render_worker_source(&plan))?;
    fs::write(output_dir.join("Cargo.toml"), render_worker_cargo(&plan))?;
    fs::write(output_dir.join("README.md"), render_worker_readme(&plan))?;

    Ok(GeneratedWorker {
        output_dir,
        worker_path,
        manifest_path,
        plan,
    })
}

pub fn verify_worker(output_dir: impl AsRef<Path>) -> Result<VerificationReport> {
    let output_dir = output_dir.as_ref();
    let manifest_path = output_dir.join("artifact.manifest.json");
    let worker_path = output_dir.join("src/main.rs");
    let plan: WorkerPlan = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    let worker_source = fs::read_to_string(&worker_path)?;
    let missing_registrations = plan
        .functions
        .iter()
        .filter_map(|function| {
            if worker_source.contains(&function.function_id) {
                None
            } else {
                Some(function.function_id.clone())
            }
        })
        .collect::<Vec<_>>();

    Ok(VerificationReport {
        ok: missing_registrations.is_empty(),
        worker_path,
        function_count: plan.functions.len(),
        missing_registrations,
    })
}

fn infer_functions(input: &ArtifactInput) -> Vec<String> {
    if !input.functions.is_empty() {
        return input.functions.clone();
    }
    let haystack = format!(
        "{} {} {}",
        input.goal.as_deref().unwrap_or_default(),
        input.source.as_deref().unwrap_or_default(),
        input.name
    )
    .to_lowercase();

    let name = input.name.to_lowercase();
    if name.contains("hackernews") || name == "hn" || haystack.contains("top stories") {
        return vec![
            "top_stories".into(),
            "get_item".into(),
            "search_cached_stories".into(),
        ];
    }
    if haystack.contains("issue") || haystack.contains("linear") || haystack.contains("jira") {
        return vec![
            "list_items".into(),
            "blocked_items".into(),
            "risk_summary".into(),
        ];
    }
    if haystack.contains("search") || haystack.contains("docs") {
        return vec![
            "search".into(),
            "get_document".into(),
            "answer_with_sources".into(),
        ];
    }
    if haystack.contains("github") || haystack.contains("repo") || haystack.contains("pull request")
    {
        return vec![
            "repo_summary".into(),
            "stale_prs".into(),
            "open_issues".into(),
        ];
    }
    vec!["inspect".into(), "list".into(), "get".into()]
}

fn plan_function(namespace: &str, function: &str) -> WorkerFunctionPlan {
    let clean = slugify(function);
    let sync_like = clean.contains("sync") || clean.contains("refresh");
    WorkerFunctionPlan {
        function_id: format!("{}::{}", namespace, clean),
        purpose: format!("{} for the {} worker", titleize(&clean), namespace),
        side_effects: if sync_like {
            SideEffects::Sync
        } else {
            SideEffects::ExternalCall
        },
        inputs: if sync_like {
            serde_json::json!({ "force": "boolean optional; bypass cache when true" })
        } else {
            serde_json::json!({ "query": "string/object; focused request payload for this function" })
        },
        output: serde_json::json!({
            "ok": "boolean success flag",
            "data": "function-specific result payload",
            "sources": "optional source/provenance list"
        }),
    }
}

fn render_worker_source(plan: &WorkerPlan) -> String {
    let registrations = plan
        .functions
        .iter()
        .map(|function| {
            format!(
                r#"    // iii.register_function(RegisterFunction::new("{function_id}", {handler_name}));
    // TODO: implement {purpose}
"#,
                function_id = function.function_id,
                handler_name = function.function_id.replace("::", "_"),
                purpose = function.purpose
            )
        })
        .collect::<String>();

    format!(
        r#"//! Generated Rust iii worker scaffold for {worker_name}.
//!
//! Wire this file to `iii-sdk` once runtime registration is enabled for this worker.
//! Function IDs are kept explicit so verification can catch drift.

fn main() {{
    let engine_url = std::env::var("III_URL").unwrap_or_else(|_| "ws://localhost:49134".to_string());
    println!("starting {worker_name} against {{engine_url}}");
{registrations}}}
"#,
        worker_name = plan.worker_name,
        registrations = registrations
    )
}

fn render_worker_cargo(plan: &WorkerPlan) -> String {
    format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
# iii-sdk = "latest"
"#,
        plan.worker_name
    )
}

fn render_worker_readme(plan: &WorkerPlan) -> String {
    let functions = plan
        .functions
        .iter()
        .map(|function| format!("- `{}` — {}", function.function_id, function.purpose))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "# {}\n\nGenerated by artifact-cli.\n\n## Functions\n\n{}\n",
        plan.worker_name, functions
    )
}

fn slugify(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_sep = false;
    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_sep = false;
        } else if !last_was_sep && !out.is_empty() {
            out.push('_');
            last_was_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "artifact".into()
    } else {
        out
    }
}

fn titleize(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
