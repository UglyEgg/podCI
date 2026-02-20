// SPDX-License-Identifier: MIT OR Apache-2.0

use super::*;

#[test]
fn search_roots_include_system_path_last() {
    let cwd = std::path::Path::new("/tmp");
    let roots = template_search_roots(cwd, None).unwrap();
    assert!(roots
        .last()
        .unwrap()
        .to_string_lossy()
        .ends_with("/usr/share/podci/templates"));
}

#[test]
fn list_includes_embedded_generic() {
    let roots: Vec<PathBuf> = vec![PathBuf::from("/this/does/not/exist")];
    let list = list_templates(&roots).unwrap();
    assert!(list.iter().any(|t| t.name == "generic"));
    let g = resolve_template(&roots, "generic").unwrap();
    assert_eq!(g.origin, TemplateOrigin::Embedded);
}

#[test]
fn disk_template_beats_embedded_generic() {
    let root = std::env::temp_dir().join(format!(
        "podci-templates-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("generic")).unwrap();
    std::fs::write(
        root.join("generic").join("template.toml"),
        "name = \"generic\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("generic").join("files")).unwrap();
    std::fs::write(
        root.join("generic").join("files").join("podci.toml"),
        "project=\"REPLACE_ME\"\n",
    )
    .unwrap();

    let roots = vec![root.clone()];
    let g = resolve_template(&roots, "generic").unwrap();
    assert!(matches!(g.origin, TemplateOrigin::Disk(_)));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn init_refuses_non_empty_dir() {
    let roots: Vec<PathBuf> = vec![];
    let dir = std::env::temp_dir().join(format!("podci-init-nonempty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("already.txt"), "x").unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let err = rt
        .block_on(init_template(&roots, "generic", &dir, "proj"))
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("must be empty"),
        "expected empty-dir refusal, got: {err:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_embedded_generic_contains_expected_paths() {
    let roots: Vec<PathBuf> = vec![];
    let mut buf = Vec::new();
    export_template_tar_gz(&roots, "generic", &mut buf).unwrap();

    let dec = flate2::read::GzDecoder::new(&buf[..]);
    let mut ar = tar::Archive::new(dec);
    let mut paths: Vec<String> = ar
        .entries()
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            e.path().unwrap().to_string_lossy().into_owned()
        })
        .collect();
    paths.sort();

    assert_eq!(
        paths,
        vec![
            "generic/files/podci.toml".to_string(),
            "generic/template.toml".to_string()
        ]
    );
}

#[test]
fn export_to_path_refuses_overwrite_and_creates_file() {
    let roots: Vec<PathBuf> = vec![];
    let dir = std::env::temp_dir().join(format!("podci-export-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let out = dir.join("generic.tar.gz");
    export_template_tar_gz_to_path(&roots, "generic", &out).unwrap();
    assert!(out.is_file());

    // Refuse overwrite.
    let err = export_template_tar_gz_to_path(&roots, "generic", &out).unwrap_err();
    assert!(
        format!("{err:?}").contains("already exists"),
        "expected overwrite refusal, got: {err:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
