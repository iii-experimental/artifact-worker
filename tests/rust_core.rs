use artifact_cli::{
    artifact_manifest, generate_worker, inspect_artifact, plan_worker, registered_function_ids,
    verify_worker, worker_metadata, ArtifactInput, SourceType, VerifyWorkerInput,
};

#[test]
fn plans_narrow_worker_functions_from_explicit_artifact_input() {
    let input = ArtifactInput {
        name: "hackernews".into(),
        goal: Some("give agents focused access to top stories and item lookup".into()),
        source_type: Some(SourceType::Docs),
        source: Some("https://github.com/HackerNews/API".into()),
        functions: vec![
            "top_stories".into(),
            "get_item".into(),
            "search_cached_stories".into(),
        ],
        output_dir: None,
    };

    let plan = plan_worker(input.clone()).unwrap();

    assert_eq!(plan.worker_name, "hackernews-worker");
    assert_eq!(plan.namespace, "hackernews");
    assert_eq!(plan.functions.len(), 3);
    assert_eq!(plan.functions[0].function_id, "hackernews::top_stories");
    assert!(plan.uses_workers.contains(&"iii-sandbox".to_string()));
}

#[test]
fn inspect_artifact_recommends_narrow_not_generic_wrapper() {
    let input = ArtifactInput {
        name: "github repo".into(),
        goal: Some("repo and pull request risk checks".into()),
        source_type: Some(SourceType::OpenApi),
        source: None,
        functions: vec![],
        output_dir: None,
    };

    let inspected = inspect_artifact(input).unwrap();

    assert_eq!(inspected.namespace, "github_repo");
    assert!(inspected.recommendation.contains("narrow iii worker"));
    assert!(inspected
        .suggested_functions
        .iter()
        .any(|id| id == "github_repo::stale_prs"));
}

#[test]
fn infers_hackernews_functions_from_name_before_source_url_noise() {
    let input = ArtifactInput {
        name: "hackernews".into(),
        goal: Some("give agents focused access to top stories and item lookup".into()),
        source_type: Some(SourceType::Docs),
        source: Some("https://github.com/HackerNews/API".into()),
        functions: vec![],
        output_dir: None,
    };

    let plan = plan_worker(input).unwrap();

    assert_eq!(plan.functions[0].function_id, "hackernews::top_stories");
    assert!(plan
        .functions
        .iter()
        .any(|function| function.function_id == "hackernews::get_item"));
}

#[test]
fn manifest_matches_old_artifact_manifest_function_surface() {
    let input = ArtifactInput {
        name: "hackernews".into(),
        goal: Some("focused agent access to top stories".into()),
        source_type: Some(SourceType::Docs),
        source: Some("https://github.com/HackerNews/API".into()),
        functions: vec!["top_stories".into(), "get_item".into()],
        output_dir: None,
    };

    let manifest = artifact_manifest(input).unwrap();

    assert_eq!(manifest.schema, "artifact-cli.manifest.preview.v1");
    assert_eq!(manifest.worker_name, "hackernews-worker");
    assert_eq!(manifest.functions.len(), 2);
    assert!(manifest.uses_workers.contains(&"iii-sandbox".to_string()));
}

#[test]
fn exposes_the_same_artifact_function_ids_as_iii_primitives() {
    assert_eq!(
        registered_function_ids(),
        vec![
            "artifact::inspect",
            "artifact::plan_worker",
            "artifact::generate_worker",
            "artifact::verify_worker",
            "artifact::manifest",
        ]
    );

    let metadata = worker_metadata();
    assert_eq!(metadata.runtime, "rust");
    assert_eq!(metadata.name, "artifact-cli-worker");
}

#[test]
fn generates_and_verifies_rust_worker_scaffold_using_iii_sdk_apis() {
    let tmp = tempfile::tempdir().unwrap();
    let input = ArtifactInput {
        name: "hackernews".into(),
        goal: Some("focused agent access to top stories".into()),
        source_type: Some(SourceType::Docs),
        source: Some("https://github.com/HackerNews/API".into()),
        functions: vec!["top_stories".into(), "get_item".into()],
        output_dir: Some(tmp.path().to_path_buf()),
    };

    let generated = generate_worker(input).unwrap();
    assert!(generated.worker_path.ends_with("src/main.rs"));
    assert!(generated.manifest_path.ends_with("artifact.manifest.json"));

    let worker_source = std::fs::read_to_string(&generated.worker_path).unwrap();
    assert!(worker_source.contains("use iii_sdk::{register_worker, InitOptions, RegisterFunction"));
    assert!(worker_source.contains("iii.register_function(RegisterFunction::new"));
    assert!(!worker_source.contains("// iii.register_function"));

    let verified = verify_worker(VerifyWorkerInput {
        output_dir: tmp.path().to_path_buf(),
    })
    .unwrap();
    assert!(verified.ok, "missing: {:?}", verified.missing_registrations);
    assert_eq!(verified.function_count, 2);
}
