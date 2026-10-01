use anyhow::Context;
use clap::Parser;
use std::env::current_dir;
use std::path::PathBuf;

use crate::runners::GraphiteRunnerContext;

mod find_svgs;
mod git_utils;
mod runners;

/// Program to generate a report on SVG import compatibility
#[derive(Parser, Debug)]
struct Args {
	/// Where to produce the output html
	#[arg(default_value = "target/svg-import-tests/report.html")]
	output: String,
	/// Directory to store cached data (e.g. downloaded test suites)
	#[arg(long, default_value = "target/svg-import-tests/cache")]
	cache_dir: String,
}

async fn run_all(parent: PathBuf) -> anyhow::Result<()> {
	println!("Exploring {parent:?}");
	let mut context = GraphiteRunnerContext::new().await;

	for file in find_svgs::FindSVGFiles::new(parent) {
		runners::run(&file.full_path, &mut context).await.context(anyhow::anyhow!("running file {file:?}"))?;
	}

	Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
	env_logger::builder().init();

	let args = Args::parse();
	let current_dir = current_dir().context("get working directory")?;
	let output = current_dir.join(args.output);
	let cache = current_dir.join(args.cache_dir);

	std::fs::create_dir_all(&cache).context(format!("create cache {}", cache.display()))?;
	if let Some(parent) = output.parent() {
		std::fs::create_dir_all(parent).context(format!("create output {}", parent.display()))?;
	}
	let resvg = git_utils::checkout(&cache, "https://github.com/linebender/resvg-test-suite.git", "rsvg-test", &["/tests"]).context("clone rsvg tests")?;
	git_utils::checkout(&cache, "https://github.com/web-platform-tests/wpt.git", "wpt", &["/svg"]).context("clone wpt")?;
	git_utils::checkout(&cache, "https://gitlab.com/inkscape/inkscape.git", "inkscape", &["/share/examples", "/testfiles/rendering_tests"]).context("clone inkscape")?;

	// run_all(resvg).await?;
	let mut context = GraphiteRunnerContext::new().await;
	context.message_trace();
	runners::run(&PathBuf::from("target/svg-import-tests/cache/rsvg-test/tests/shapes/circle/simple-case.svg"), &mut context).await?;
	Ok(())
}
