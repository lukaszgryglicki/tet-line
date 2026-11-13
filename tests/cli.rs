use std::process::Command;

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tet-line"));
    command.env_remove("DISPLAY").env_remove("WAYLAND_DISPLAY");
    command
}

#[test]
fn help_and_argument_errors_do_not_need_a_display_or_tet() {
    let directory = tempfile::tempdir().unwrap();
    let help = command()
        .env("PATH", directory.path())
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage: tet-line"));
    assert!(help.stderr.is_empty());
    for args in [
        vec![],
        vec!["2", "NaN", "0", "0", "1", "0"],
        vec!["2", "0", "0", "0", "1", "0", "1"],
        vec!["x^^", "0", "0", "1", "0"],
        vec!["sin(x,x)", "0", "0", "1", "0"],
        vec!["--expr=", "0", "0", "1", "0"],
        vec!["x", "0", "NaN", "1", "0"],
    ] {
        let output = command()
            .env("PATH", directory.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.starts_with("error:"));
        assert!(!stderr.contains("Calculating"));
    }
}

#[test]
fn missing_tet_fails_before_opening_a_window() {
    let directory = tempfile::tempdir().unwrap();
    for args in [
        vec!["2", "0", "0", "0", "1", "0", "2"],
        vec!["x^^x-x^x+x", "1", "0", "2", "0", "2"],
        vec!["ate(2,x)-ssrt(x,2)", "1", "0", "2", "0", "2"],
    ] {
        let output = command()
            .env("PATH", directory.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr)
            .contains("ensure the tet executable is on PATH"));
    }
}

#[cfg(unix)]
#[test]
fn all_failed_samples_report_the_solver_error_without_opening_a_window() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("tet");
    std::fs::write(
        &program,
        "#!/bin/sh\necho 'unsupported height' >&2\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    for args in [
        vec!["2", "0", "-3", "0", "-2", "0", "2"],
        vec!["sin(x^^x)+x", "-3", "0", "-2", "0", "2"],
    ] {
        let output = command()
            .env("PATH", directory.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unsupported height"));
        assert!(stderr.contains("no finite values"));
    }
}

#[cfg(unix)]
#[test]
fn coordinate_expressions_run_tet_before_the_calculation_starts() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("tet");
    std::fs::write(
        &program,
        "#!/bin/sh\necho \"$@\" > \"$0.args\"\n\
         [ \"$4\" = 2 ] || { echo 'unsupported height' >&2; exit 1; }\n\
         echo '1.8371173070873836 0'\n",
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let run = |path: &std::path::Path, args: &[&str]| {
        let output = command().env("PATH", path).args(args).output().unwrap();
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };

    let (code, stderr) = run(directory.path(), &["1/0", "1.5^^2", "0", "3", "0", "2"]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("sample 1 at x=1.8371173070873836+0i"));
    assert!(stderr.contains("no finite values"));
    assert_eq!(
        std::fs::read_to_string(directory.path().join("tet.args")).unwrap(),
        "13 1.5 0 2 0\n"
    );

    let (code, stderr) = run(directory.path(), &["x", "0", "0", "1.5^^3", "0"]);
    assert_eq!(code, Some(2));
    assert!(stderr.starts_with("error: plot_to_re must be"));
    assert!(stderr.contains("got '1.5^^3': tet exited with"));
    assert!(stderr.contains("unsupported height"));

    let empty = tempfile::tempdir().unwrap();
    let (code, stderr) = run(empty.path(), &["x", "1.5^^2", "0", "3", "0"]);
    assert_eq!(code, Some(2));
    assert!(stderr.starts_with("error: plot_from_re must be"));
    assert!(stderr.contains("ensure the tet executable is on PATH"));
    assert!(!stderr.contains("Calculating"));
}

#[test]
fn all_nonfinite_expression_results_fail_without_needing_tet_or_a_display() {
    let directory = tempfile::tempdir().unwrap();
    let output = command()
        .env("PATH", directory.path())
        .args(["1/0", "0", "0", "1", "0", "2"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("expression returned a nonfinite value"));
    assert!(stderr.contains("no finite values"));
    assert!(!stderr.contains("could not run tet"));
}
