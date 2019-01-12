extern crate compiletest_rs as compiletest;

use compiletest::Config;

fn run_mode(mode: &str) {
    let mut config = Config::default();

    config.mode = mode.parse().expect("Invalid mode");
    config.src_base = format!("tests/compile_test/{}", mode).into();
    config.link_deps();
    config.clean_rmeta();

    compiletest::run_tests(&config);
}

#[test]
fn compile_test() {
    run_mode("compile-fail");
}
