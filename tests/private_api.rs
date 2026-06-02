use std::process::Command;

const ENABLE_ENV: &str = "ASFML_RUN_PRIVATE_API_TESTS";

#[test]
fn access_private_opendal_with_stored_session() {
    if !enabled() {
        return;
    }

    let status = asfml(["auth", "status", "private@opendal.apache.org"]);
    assert!(status.status.success(), "auth status failed");

    let list = asfml([
        "list",
        "private@opendal.apache.org",
        "--limit",
        "1",
        "--format",
        "json",
    ]);
    assert!(list.status.success(), "private list failed");
    let mid = first_mid(&list).expect("private list returned no mid");

    let email = asfml(["read", &mid, "--format", "json"]);
    assert!(email.status.success(), "private read failed");
    let email: serde_json::Value =
        serde_json::from_slice(&email.stdout).expect("private read returned invalid json");
    assert_eq!(email["mid"].as_str(), Some(mid.as_str()));

    let thread = asfml(["read", &mid, "--thread", "--format", "json"]);
    assert!(thread.status.success(), "private thread read failed");
    let thread: serde_json::Value =
        serde_json::from_slice(&thread.stdout).expect("private thread returned invalid json");
    assert!(thread["emails"].as_array().is_some());
}

#[test]
fn search_private_opendal_with_stored_session() {
    if !enabled() {
        return;
    }

    let search = asfml([
        "search",
        "private@opendal.apache.org",
        "Re:",
        "--limit",
        "1",
        "--format",
        "json",
    ]);
    assert!(search.status.success(), "private search failed");
    let mid = first_mid(&search).expect("private search returned no mid");

    let email = asfml(["read", &mid, "--format", "json"]);
    assert!(email.status.success(), "private search result read failed");
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

fn first_mid(output: &std::process::Output) -> Option<String> {
    let emails: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).ok()?;
    emails
        .first()
        .and_then(|email| email["mid"].as_str().or_else(|| email["id"].as_str()))
        .map(ToString::to_string)
}
