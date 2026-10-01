use anyhow::Context;
use graph_craft::application_io::{self, PlatformApplicationIo};
use graphite_editor::messages::frontend::utility_types::{ExportBounds, FileType};
use std::path::PathBuf;

use graph_craft::document::value::{RenderOutput, RenderOutputType, TaggedValue};
use graphite_editor::messages::portfolio::ingest::utility_types::IngestAction;
use graphite_editor::node_graph_executor::{ExportConfig, NodeGraphUpdate};
use graphite_editor::test_utils::test_prelude::*;

fn setup_logger() {
	tracing::subscriber::set_global_default(my_subscriber).expect("setting tracing default failed");
}

#[derive(Default)]
pub struct Pixmap {
	/// Unaligned premultiplied RGB8 TODO: gamma?
	data: Vec<u8>,
	/// [`Self::data`] is not aligned so width == stride
	width: u32,
	height: u32,
}

enum RenderResult {
	Render { pixmap: Pixmap, intermediate_svg: Option<String>, logs: String },
	Error(String),
}

fn run_resvg(content: &[u8]) -> anyhow::Result<RenderResult> {
	let tree = resvg::usvg::Tree::from_data(content, &resvg::usvg::Options::default()).context("parse resvg tree")?;
	let size = tree.size().to_int_size();
	let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).context("create pixmap")?;
	resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap.as_mut());
	let pixmap = Pixmap {
		width: pixmap.width(),
		height: pixmap.height(),
		data: pixmap.take(),
	};
	Ok(RenderResult::Render {
		pixmap,
		intermediate_svg: None,
		logs: String::new(),
	})
}

pub struct GraphiteRunnerContext {
	test_utils: EditorTestUtils,
}

impl GraphiteRunnerContext {
	pub async fn new() -> Self {
		let application_io = PlatformApplicationIo::new().await;
		let test_utils = EditorTestUtils::create_with_application_io(application_io);
		Self { test_utils }
	}

	pub fn message_trace(&mut self) {
		self.test_utils.editor.handle_message(DebugMessage::MessageNames);
	}

	async fn import_svg(&mut self, content: Vec<u8>) -> anyhow::Result<()> {
		let frontend_responses = self
			.test_utils
			.handle_message(IngestMessage::Ingest {
				data: content,
				action: IngestAction::Open,
				mime_type: "image/svg+xml".to_string(),
				path: None,
			})
			.await;
		for response in &frontend_responses {
			response.check_node_graph_error();
		}
		Ok(())
	}
	async fn execute_export(&mut self, use_svg: bool) -> anyhow::Result<Pixmap> {
		let editor = &mut self.test_utils;
		let document = editor.portfolio_message_handler().active_document().ok_or(anyhow::anyhow!("No active document"))?;
		let mut top_level_layers = LayerNodeIdentifier::ROOT_PARENT.children(document.metadata());
		let [Some(artboard), None] = [top_level_layers.next(), top_level_layers.next()] else {
			return Err(anyhow::anyhow!("expect exactly one top level layer"));
		};
		if !document.network_interface.is_artboard(&artboard.to_node(), &[]) {
			return Err(anyhow::anyhow!("top level layer should be artboard"));
		}

		let portfolio = editor.portfolio_message_handler_mut();
		let document_id = portfolio.active_document_id().ok_or(anyhow::anyhow!("no active document"))?;
		let document = portfolio.documents.get_mut(&document_id).ok_or(anyhow::anyhow!("no active document"))?;
		let export_config = ExportConfig {
			scale_factor: 1.,
			bounds: ExportBounds::Artboard(artboard),
			// The file type for raster doesn't matter
			file_type: if use_svg { FileType::Svg } else { FileType::Bmp },
			..Default::default()
		};
		portfolio
			.executor
			.submit_document_export(document, document_id, export_config)
			.map_err(|e| anyhow::anyhow!("submit export: {e}"))?;
		editor.runtime.run().await;
		let results = editor.portfolio_message_handler_mut().executor.poll_raw_results();

		let result = results
			.into_iter()
			.find_map(|result| if let NodeGraphUpdate::ExecutionResponse(response) = result { Some(response) } else { None });
		let node_graph_output = result
			.ok_or(anyhow::anyhow!("no execution response after submitting export"))?
			.result
			.map_err(|e| anyhow::anyhow!("execute: {e}"))?;
		match node_graph_output {
			TaggedValue::RenderOutput(RenderOutput {
				data: RenderOutputType::Svg { svg, .. },
				..
			}) => {
				let mut result = run_resvg(svg.as_bytes()).context(anyhow::anyhow!("running resvg on graphite ouput:\n{svg}"))?;
				result.intermediate_svg = Some(svg);
				Ok(result)
			}
			TaggedValue::RenderOutput(RenderOutput {
				data: RenderOutputType::Buffer { data, width, height },
				..
			}) => Ok(Pixmap {
				data,
				width,
				height,
				..Default::default()
			}),
			_ => Err(anyhow::anyhow!("Incorrect type from execution response {node_graph_output})")),
		}
	}
	fn close_documents(&mut self) {
		self.test_utils.editor.handle_message(PortfolioMessage::CloseAllDocuments);
	}

	async fn run_graphite(&mut self, content: Vec<u8>) -> anyhow::Result<Pixmap> {
		self.import_svg(content).await.context("import svg to graphite")?;
		let pixmap = self.execute_export().await.context("running graph")?;
		self.close_documents();
		Ok(pixmap)
	}
}

pub async fn run(svg: &PathBuf, graphite_runner: &mut GraphiteRunnerContext) -> anyhow::Result<()> {
	// Err(anyhow::anyhow!("bob"))
	println!("Running {svg:?}");
	let content = std::fs::read(svg).context("read SVG file")?;
	let resvg = match run_resvg(&content).context("run resvg") {
		Ok(pixmap) => pixmap,
		Err(e) => {
			eprintln!("{e:?}");
			return Ok(());
		}
	};
	let x = match graphite_runner.run_graphite(content).await {
		Ok(pixmap) => pixmap,
		Err(e) => {
			eprintln!("{e:?}");
			return Ok(());
		}
	};

	Ok(())
}
