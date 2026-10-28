use std::process::{Command, Output};

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_egyptian-fraction-search"))
        .args(arguments)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn compare_exposes_pareto_solutions_and_separate_work() {
    let output = run(&[
        "compare",
        "2/7",
        "--max-denominator",
        "35",
        "--max-terms",
        "4",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout(&output);
    assert!(text.contains("Greedy: 2/7 = 1/4 + 1/28"));
    assert!(text.contains("1/6 + 1/14 + 1/21"));
    assert!(text.contains("Search status: optimal within bounds"));
    assert!(text.contains("Greedy work: denominators examined:"));
    assert!(text.contains("Search work: denominators examined:"));
    for metric in [
        "largest denominator:",
        "search nodes:",
        "search branches:",
        "arithmetic operations:",
        "runtime:",
    ] {
        assert!(text.contains(metric), "{metric}");
    }
}

#[test]
fn greedy_can_be_valid_while_exceeding_the_search_bounds() {
    let output = run(&[
        "compare",
        "4/5",
        "--max-denominator",
        "10",
        "--max-terms",
        "3",
    ]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("Greedy within search bounds: no"));
    assert!(text.contains("Solution 1: 4/5 = 1/2 + 1/5 + 1/10"));
}

#[test]
fn incomplete_search_has_an_explicit_status_and_exit_code() {
    let output = run(&["search", "2/7", "--max-nodes", "0"]);
    assert_eq!(output.status.code(), Some(3));
    let text = stdout(&output);
    assert!(text.contains("budget exhausted; search incomplete"));
    assert!(!text.contains("optimal within bounds"));
    assert!(!text.contains("no solution within bounds"));

    let output = run(&[
        "search",
        "2/7",
        "--max-denominator",
        "35",
        "--max-terms",
        "4",
        "--max-nodes",
        "3",
    ]);
    assert_eq!(output.status.code(), Some(3));
    assert!(stdout(&output).contains("Best found 1: 2/7 = 1/4 + 1/28"));
}

#[test]
fn completed_empty_search_is_distinct_from_incomplete_search() {
    let output = run(&[
        "search",
        "2/7",
        "--max-denominator",
        "10",
        "--max-terms",
        "2",
    ]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Search status: no solution within bounds"));
}

#[test]
fn objectives_and_invalid_inputs_are_checked() {
    let output = run(&[
        "search",
        "2/7",
        "--objective",
        "denominator",
        "--max-denominator",
        "35",
        "--max-terms",
        "4",
    ]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Solution 1: 2/7 = 1/6 + 1/14 + 1/21"));
    for arguments in [
        vec!["greedy", "4/3"],
        vec!["search", "1/201"],
        vec!["search", "2/7", "--objective", "fastest"],
        vec!["search", "2/7", "--max-terms", "9"],
        vec!["greedy", "2/7", "--max-bits", "0"],
    ] {
        let output = run(&arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn greedy_limits_report_a_verified_prefix() {
    let output = run(&["greedy", "4/17", "--max-steps", "1"]);
    assert_eq!(output.status.code(), Some(3));
    let text = stdout(&output);
    assert!(text.contains("Greedy status: incomplete (step limit)"));
    assert!(text.contains("Greedy prefix: 1/5"));
    assert!(text.contains("Remainder: 3/85"));
    assert!(text.contains("Verified: prefix + remainder = 4/17"));
    assert!(!text.contains("Greedy: 4/17 = 1/5"));
}

#[test]
fn verify_command_checks_distinctness_and_exact_equality() {
    let output = run(&["verify", "2/7", "6,14,21"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("Verified: 2/7 = 1/6 + 1/14 + 1/21"));
    for denominators in [vec!["4", "27"], vec!["7", "7"], vec!["0"]] {
        let mut arguments = vec!["verify", "2/7"];
        arguments.extend(denominators);
        assert_eq!(run(&arguments).status.code(), Some(2));
    }
}
