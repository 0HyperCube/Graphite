use anyhow::Context;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct SvgFile {
	pub full_path: PathBuf,
	pub parent_name: String,
	pub relative: PathBuf,
}

pub struct FindSVGFiles {
	dir_stack: Vec<PathBuf>,
	entires: Option<std::fs::ReadDir>,
	base_dir: Vec<PathBuf>,
}

impl FindSVGFiles {
	pub fn new(directory: PathBuf) -> Self {
		Self {
			dir_stack: vec![directory.clone()],
			entires: None,
			base_dir: vec![directory],
		}
	}

	fn find_relative_and_base(&mut self, path: &Path) -> Option<(String, PathBuf)> {
		while let Some(base) = self.base_dir.last() {
			if let Ok(relative) = path.strip_prefix(base)
				&& let Some(parent_name) = base.components().next_back().and_then(|last| last.as_os_str().to_str())
			{
				return Some((parent_name.to_string(), relative.to_path_buf()));
			}
			self.base_dir.pop();
		}
		None
	}
}

impl Iterator for FindSVGFiles {
	type Item = SvgFile;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			if let Some(entry) = self.entires.as_mut().and_then(|entries| entries.next()) {
				let entry = entry.context(anyhow::anyhow!("read entry")).unwrap();
				let meta = entry.metadata().context(anyhow::anyhow!("read metadata for {:?}", entry.path())).unwrap();
				let full_path = entry.path();
				let parent = full_path.parent().expect("entry should have parent");

				if meta.is_dir() && full_path.ends_with(".git") {
					self.base_dir.push(parent.to_path_buf());
				}

				if meta.is_dir() {
					self.dir_stack.push(full_path.clone());
				}
				if meta.is_file() && full_path.extension().is_some_and(|ext| ext.to_str() == Some("svg")) {
					let (parent_name, relative) = self.find_relative_and_base(&full_path).expect("find relative and base");
					return Some(SvgFile { full_path, parent_name, relative });
				}
			} else {
				let path = self.dir_stack.pop()?;
				self.entires = Some(std::fs::read_dir(&path).context(anyhow::anyhow!("read {path:?}")).unwrap());
			}
		}
	}
}
