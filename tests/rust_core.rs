use artifact_cli::{
    generate_worker, inspect_artifact, plan_worker, verify_worker, ArtifactInput, SourceType,
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

    let plan = plan_worker(&input);

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

    let inspected = inspect_artifact(&input);

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

    let plan = plan_worker(&input);

    assert_eq!(plan.functions[0].function_id, "hackernews::top_stories");
    assert!(plan
        .functions
        .iter()
        .any(|function| function.function_id == "hackernews::get_item"));
}

#[test]
fn generates_and_verifies_rust_worker_scaffold() {
    let tmp = tempfile::tempdir().unwrap();
    let input = ArtifactInput {
        name: "hackernews".into(),
        goal: Some("focused agent access to top stories".into()),
        source_type: Some(SourceType::Docs),
        source: Some("https://github.com/HackerNews/API".into()),
        functions: vec!["top_stories".into(), "get_item".into()],
        output_dir: Some(tmp.path().to_path_buf()),
    };

    let generated = generate_worker(&input).unwrap();
    assert!(generated.worker_path.ends_with("src/main.rs"));
    assert!(generated.manifest_path.ends_with("artifact.manifest.json"));

    let verified = verify_worker(tmp.path()).unwrap();
    assert!(verified.ok, "missing: {:?}", verified.missing_registrations);
    assert_eq!(verified.function_count, 2);
}
