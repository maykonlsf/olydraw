use olydraw::prefs::{ExportMode, Prefs, load_from, save_to};

#[test]
fn save_then_load_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let prefs = Prefs {
        color: [1, 2, 3, 255],
        stroke_width: 5.5,
        palette: vec![[9, 9, 9, 255], [7, 7, 7, 255]],
        hotkey: "Ctrl+Shift+X".to_string(),
        passthrough_hotkey: "Ctrl+Shift+Y".to_string(),
        export_dir: Some("/tmp/exports".to_string()),
        export_mode: ExportMode::ScreenComposite,
    };
    save_to(dir.path(), &prefs).expect("save");
    assert_eq!(load_from(dir.path()), prefs);
}

#[test]
fn missing_passthrough_hotkey_falls_back_to_default() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("prefs.toml"), "stroke_width = 9.0\n").unwrap();
    let prefs = load_from(dir.path());
    assert_eq!(
        prefs.passthrough_hotkey,
        Prefs::default().passthrough_hotkey
    );
}

#[test]
fn missing_file_returns_defaults() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(load_from(dir.path()), Prefs::default());
}

#[test]
fn missing_directory_returns_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let nonexistent = dir.path().join("does/not/exist");
    assert_eq!(load_from(&nonexistent), Prefs::default());
}

#[test]
fn corrupt_file_returns_defaults_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("prefs.toml"), "this is {{{ not toml [").unwrap();
    assert_eq!(load_from(dir.path()), Prefs::default());
}

#[test]
fn unknown_keys_are_ignored_and_known_keys_applied() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("prefs.toml"),
        "stroke_width = 9.0\nfuture_setting = \"whatever\"\n",
    )
    .unwrap();
    let prefs = load_from(dir.path());
    assert_eq!(prefs.stroke_width, 9.0);
    assert_eq!(
        prefs.color,
        Prefs::default().color,
        "unset keys fall back to defaults"
    );
}

#[test]
fn save_creates_missing_directories() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a/b/c");
    let prefs = Prefs::default();
    save_to(&nested, &prefs).expect("save into missing dirs");
    assert_eq!(load_from(&nested), prefs);
}
