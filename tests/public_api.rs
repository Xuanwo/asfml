use std::process::Command;

const ENABLE_ENV: &str = "ASFML_RUN_PUBLIC_API_TESTS";
const RELEASE_MID: &str = "qd7m1k6h9hmjt5hdqb28y3vzh561x3bj";

#[test]
fn list_public_opendal_dev() {
    if !enabled() {
        return;
    }

    let output = asfml(["list", "dev@opendal.apache.org", "--limit", "1"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains(RELEASE_MID), "{stdout}");
    assert!(
        stdout.contains("[DISCUSS] Release Apache OpenDAL v0.57.0"),
        "{stdout}"
    );
}

#[test]
fn search_public_opendal_dev() {
    if !enabled() {
        return;
    }

    let output = asfml([
        "search",
        "dev@opendal.apache.org",
        "release",
        "--limit",
        "1",
        "--format",
        "json",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("6rhj403fyfqoqzyv4201m53gqwkbqrvy"),
        "{stdout}"
    );
    assert!(stdout.contains("Component Support Tiers"), "{stdout}");
}

#[test]
fn read_public_opendal_thread() {
    if !enabled() {
        return;
    }

    let output = asfml(["read", RELEASE_MID, "--thread"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("Thread: [DISCUSS] Release Apache OpenDAL v0.57.0"),
        "{stdout}"
    );
    assert!(stdout.contains("Messages: 1"), "{stdout}");
}

fn enabled() -> bool {
    std::env::var_os(ENABLE_ENV).is_some()
}

fn asfml<const N: usize>(args: [&str; N]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_asfml"))
        .args(args)
        .output()
        .expect("failed to run asfml")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
