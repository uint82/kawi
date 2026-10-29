mod expectations;
mod harness;
mod meta;
mod normalize;
mod report;
mod runner;
mod scenario;

#[test]
fn runs_the_conformance_suite() {
    let report = runner::run();
    println!("{report}");
    assert!(
        !report.has_failures(),
        "{report}\n{:#?}",
        report.failure_messages()
    );
}
