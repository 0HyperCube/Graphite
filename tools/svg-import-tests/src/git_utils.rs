use anyhow::Context;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run a git command and capture trimmed stdout.
fn git(args: &[&str], path: impl AsRef<Path>) -> anyhow::Result<String> {
	let output = Command::new("git").current_dir(path).args(args).output().context(format!("run git command `git {}`", args.join(" ")))?;
	let stdout = String::from_utf8(output.stdout).context("stdout not utf8")?;
	let stderr = String::from_utf8(output.stderr).context("stderr not utf8")?;

	if !output.status.success() {
		return Err(anyhow::anyhow!(
			"run git command `git {}` exited with status {}:\nStdout:\n{}\nstderr:\n{}",
			args.join(" "),
			output.status,
			stdout,
			stderr
		));
	}
	Ok(stdout.trim().to_string())
}

/// Shallow check out a git repo (checking out only `subdirectory` if specified otherwise the entire repo)
pub fn checkout(parent: &PathBuf, url: &str, name: &str, subdirectory: &[&str]) -> anyhow::Result<PathBuf> {
	let output_directory = parent.join(name);
	let output_directory_str = output_directory.to_str().ok_or_else(|| anyhow::anyhow!("invalid directory {output_directory:?}"))?;
	if !output_directory.exists() {
		println!("Cloning into {output_directory_str}");
		git(&["clone", "--no-checkout", "--depth=1", "--filter=tree:0", url, output_directory_str], parent).context("clone")?;
	}
	if !subdirectory.is_empty() {
		git(&[&["sparse-checkout", "set", "--no-cone"], subdirectory].concat(), output_directory_str).context("sparse-checkout")?;
	}
	git(&["checkout"], output_directory_str).context("checkout")?;

	Ok(output_directory)
}
