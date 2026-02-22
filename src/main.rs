//! MCP server for generating perfectly formatted 80-column Rust comment
//! banners.
//!
//! Manually formatting banners is error-prone — both humans and LLMs
//! routinely miscenter the text. This server solves the problem once: call
//! the `banner` tool with text, get a correctly formatted 3-line banner
//! back.
//!
//! # Banner format
//!
//! ```text
//! ////////////////////////////////////////////////////////////////////////////////
//! //                             Banner text here.                              //
//! ////////////////////////////////////////////////////////////////////////////////
//! ```
//!
//! - **Total width**: 80 columns
//! - **Lines 1 and 3**: 80 forward slashes
//! - **Line 2**: `//` + left padding + text + right padding + `//`
//!   - Inner width: 76 characters (80 − 4 for the `//` delimiters)
//!   - `left = (76 − text_len) / 2` (integer division, floors)
//!   - `right = 76 − text_len − left` (gets the extra space when odd)
//! - Text longer than 74 characters (leaving fewer than 1 space each side)
//!   returns an error.
//!
//! # Protocol lifecycle
//!
//! ```text
//!  Client (agent)                        Server (banners-mcp)
//!       │                                       │
//!       │──── initialize ──────────────────────►│
//!       │◄─── initialize response ──────────────│
//!       │──── initialized ─────────────────────►│
//!       │                                       │
//!       │──── tools/list ──────────────────────►│
//!       │◄─── tool schemas ─────────────────────│
//!       │                                       │
//!       │──── tools/call { banner } ───────────►│
//!       │◄─── 3-line banner ────────────────────│
//!       │                                       │
//!       │──── (close stdin / Ctrl-C) ──────────►│
//! ```

use rmcp::{
	ErrorData as McpError, ServerHandler, ServiceExt,
	handler::server::{router::tool::ToolRouter, wrapper::Parameters},
	model::*,
	tool, tool_handler, tool_router,
	transport::stdio
};
use serde::Deserialize;

////////////////////////////////////////////////////////////////////////////////
//                             Banner formatting.                             //
////////////////////////////////////////////////////////////////////////////////

/// Total width of the banner in columns.
const BANNER_WIDTH: usize = 80;

/// Maximum text length that still fits with at least one space of padding
/// on each side (80 − 4 delimiter chars − 2 minimum spaces = 74).
const MAX_TEXT_LEN: usize = BANNER_WIDTH - 4 - 2;

/// Inner width between the `//` delimiters (80 − 4 = 76).
const INNER_WIDTH: usize = BANNER_WIDTH - 4;

/// Format text into a correctly padded 80-column Rust comment banner.
///
/// Produces a 3-line banner:
/// 1. 80 forward slashes
/// 2. `//` + left padding + text + right padding + `//`
/// 3. 80 forward slashes
///
/// Left padding floors the division; right padding gets the extra space
/// when the text length is odd.
///
/// # Parameters
///
/// * `text` — the banner text. Must be ≤ 74 characters.
///
/// # Returns
///
/// The 3-line banner as a [`String`].
///
/// # Errors
///
/// Returns [`McpError`] if `text` exceeds 74 characters.
fn format_banner(text: &str) -> Result<String, McpError>
{
	let text_len = text.len();
	if text_len > MAX_TEXT_LEN
	{
		return Err(McpError::invalid_params(
			format!(
				"text is {text_len} characters, maximum is {MAX_TEXT_LEN} \
				 (must leave at least 1 space of padding on each side)"
			),
			None
		));
	}

	let left = (INNER_WIDTH - text_len) / 2;
	let right = INNER_WIDTH - text_len - left;

	let bar = "/".repeat(BANNER_WIDTH);
	let middle = format!(
		"//{left}{text}{right}//",
		left = " ".repeat(left),
		right = " ".repeat(right),
	);

	Ok(format!("{bar}\n{middle}\n{bar}"))
}

////////////////////////////////////////////////////////////////////////////////
//                             Server definition.                             //
////////////////////////////////////////////////////////////////////////////////

/// The banners MCP server.
///
/// Implements [`ServerHandler`] to respond to MCP protocol requests. The
/// server advertises a single `banner` tool and dispatches calls through a
/// [`ToolRouter`].
///
/// # Notes
///
/// The struct must be [`Clone`] because the rmcp framework may clone it
/// during service setup.
#[derive(Clone)]
struct BannerServer
{
	/// Routes incoming `tools/call` requests to the `banner` handler.
	/// Populated by the [`tool_router`] macro from the `#[tool]`-annotated
	/// method.
	tool_router: ToolRouter<Self>
}

////////////////////////////////////////////////////////////////////////////////
//                           Tool parameter types.                            //
////////////////////////////////////////////////////////////////////////////////

/// Request payload for the `banner` tool.
///
/// Contains a single `text` field with the banner text to center.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct BannerRequest
{
	/// The text to center in the banner. Must be 74 characters or fewer.
	#[schemars(description = "The text to center in the banner. Must be 74 \
			characters or fewer.")]
	text: String
}

////////////////////////////////////////////////////////////////////////////////
//                             Tool definitions.                              //
////////////////////////////////////////////////////////////////////////////////

/// Tool definitions for the banners MCP server.
///
/// The single `#[tool]`-annotated method is automatically registered in the
/// [`ToolRouter`] and exposed to MCP clients via `tools/list`.
///
/// # Available tools
///
/// - **`banner`** — format text into an 80-column Rust comment banner.
#[tool_router]
impl BannerServer
{
	/// Create a new banners MCP server.
	///
	/// Initializes the tool router from the `#[tool]`-annotated methods
	/// in this `impl` block.
	pub fn new() -> Self
	{
		Self {
			tool_router: Self::tool_router()
		}
	}

	/// Format text into an 80-column Rust comment banner.
	///
	/// Produces a 3-line banner with the text centered between `//`
	/// delimiters. When the text length is odd relative to the inner
	/// width, the extra space goes on the right.
	///
	/// # Parameters
	///
	/// * `req` — a [`BannerRequest`] containing the text to center.
	///
	/// # Returns
	///
	/// A [`CallToolResult`] with a single text content block containing
	/// the 3-line banner.
	///
	/// # Errors
	///
	/// Returns [`McpError`] if the text exceeds 74 characters.
	#[tool(
		name = "banner",
		description = "Format text into a perfectly centered 80-column \
			Rust comment banner. Returns a 3-line banner: a line of 80 \
			slashes, the text centered between // delimiters with space \
			padding, and another line of 80 slashes. Text must be 74 \
			characters or fewer."
	)]
	fn banner(
		&self,
		Parameters(req): Parameters<BannerRequest>
	) -> Result<CallToolResult, McpError>
	{
		let banner = format_banner(&req.text)?;
		Ok(CallToolResult::success(vec![Content::text(banner)]))
	}
}

////////////////////////////////////////////////////////////////////////////////
//                           MCP protocol handler.                            //
////////////////////////////////////////////////////////////////////////////////

/// MCP protocol handler for the banners server.
///
/// The [`tool_handler`] attribute auto-generates [`ServerHandler::call_tool`]
/// and [`ServerHandler::list_tools`] implementations by delegating to
/// `self.tool_router`. Only [`get_info`](ServerHandler::get_info) is
/// manually implemented — no resources are served.
#[tool_handler]
impl ServerHandler for BannerServer
{
	/// Return server metadata for the MCP initialize handshake.
	///
	/// Reports the server's name, version, protocol version, and
	/// capabilities (tools only — no resources).
	fn get_info(&self) -> ServerInfo
	{
		ServerInfo {
			protocol_version: ProtocolVersion::V_2024_11_05,
			capabilities: ServerCapabilities::builder().enable_tools().build(),
			server_info: Implementation {
				name: env!("CARGO_PKG_NAME").into(),
				version: env!("CARGO_PKG_VERSION").into(),
				..Default::default()
			},
			instructions: Some(
				"Banner formatting tool. Call the `banner` tool with a \
				 `text` parameter (≤74 characters) to get a perfectly \
				 centered 80-column Rust comment banner."
					.into()
			)
		}
	}
}

////////////////////////////////////////////////////////////////////////////////
//                             Main entry point.                              //
////////////////////////////////////////////////////////////////////////////////

/// Launch the banners MCP server over stdio.
///
/// Initializes the tracing subscriber for diagnostic logging (to stderr —
/// stdout is reserved for the MCP protocol), creates the server, and
/// serves it until the client disconnects.
///
/// # Errors
///
/// Returns an error if the tracing subscriber cannot be initialized or the
/// MCP service encounters a fatal error during operation.
#[tokio::main]
async fn main() -> anyhow::Result<()>
{
	tracing_subscriber::fmt()
		.with_writer(std::io::stderr)
		.with_ansi(true)
		.with_max_level(tracing::Level::INFO)
		.init();

	tracing::info!("starting banners-mcp server (stdio transport)");

	let server = BannerServer::new();
	let service = server
		.serve(stdio())
		.await
		.inspect_err(|e| tracing::error!("serving error: {e}"))?;

	service.waiting().await?;
	tracing::info!("banners-mcp server shut down");
	Ok(())
}

////////////////////////////////////////////////////////////////////////////////
//                                   Tests.                                   //
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests
{
	use rmcp::{ClientHandler, ServiceExt, model::CallToolRequestParams};

	use super::*;

	/// A no-op MCP client handler used to drive the server in tests.
	///
	/// The [`ClientHandler`] trait requires no methods to be implemented —
	/// the default implementations handle everything. This type exists
	/// solely to satisfy the trait bound on
	/// [`ServiceExt::serve`](rmcp::ServiceExt::serve).
	#[derive(Default, Clone)]
	struct TestClient;

	impl ClientHandler for TestClient {}

	/// Convenience alias for the running MCP client service.
	type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

	/// Extract the text string from the first content block of a tool
	/// result.
	///
	/// # Parameters
	///
	/// * `result` — the [`CallToolResult`] returned by `call_tool`.
	///
	/// # Returns
	///
	/// A `&str` reference to the text payload.
	///
	/// # Panics
	///
	/// Panics if the first content block is not a [`RawContent::Text`]
	/// variant.
	fn first_text(result: &CallToolResult) -> &str
	{
		match &result.content[0].raw
		{
			RawContent::Text(text) => &text.text,
			other => panic!("expected text content, got {other:?}")
		}
	}

	/// Create an in-process MCP client–server pair connected via a
	/// [`tokio::io::duplex`] channel.
	///
	/// ```text
	///  TestClient ◄──── duplex channel ────► BannerServer
	///  (client)         (4096-byte buf)       (server task)
	/// ```
	///
	/// # Returns
	///
	/// A [`Client`] handle connected to the running server.
	///
	/// # Panics
	///
	/// Panics if either side fails to initialize.
	async fn setup() -> Client
	{
		let server = BannerServer::new();
		let (server_transport, client_transport) = tokio::io::duplex(4096);
		tokio::spawn(async move {
			let service = server.serve(server_transport).await.unwrap();
			service.waiting().await.unwrap();
		});
		TestClient
			.serve(client_transport)
			.await
			.expect("client failed to connect")
	}

	/// Helper to call the `banner` tool with the given text.
	///
	/// # Parameters
	///
	/// * `client` — the running MCP client.
	/// * `text` — the banner text to send.
	///
	/// # Returns
	///
	/// The [`CallToolResult`] from the server.
	async fn call_banner(
		client: &Client,
		text: &str
	) -> Result<CallToolResult, rmcp::ServiceError>
	{
		client
			.call_tool(CallToolRequestParams {
				meta: None,
				name: "banner".into(),
				arguments: Some(
					serde_json::json!({"text": text})
						.as_object()
						.unwrap()
						.clone()
				),
				task: None
			})
			.await
	}

	/// Verify that the server advertises exactly one tool named
	/// `banner` with a description and non-empty input schema.
	#[tokio::test]
	async fn tools_are_listed()
	{
		let client = setup().await;
		let tools = client.list_all_tools().await.unwrap();
		assert_eq!(tools.len(), 1, "expected exactly 1 tool");
		assert_eq!(tools[0].name, "banner");
		assert!(
			tools[0].description.is_some(),
			"banner tool has no description"
		);
		assert!(
			!tools[0].input_schema.is_empty(),
			"banner tool has empty input schema"
		);
	}

	/// Verify correct banner for even-padding text.
	///
	/// `"Banner text here."` is 17 characters. Inner width is 76.
	/// Left = (76 − 17) / 2 = 29, right = 76 − 17 − 29 = 30.
	#[tokio::test]
	async fn banner_even_padding()
	{
		let client = setup().await;
		let result = call_banner(&client, "Banner text here.").await.unwrap();
		let text = first_text(&result);
		let lines: Vec<&str> = text.lines().collect();
		assert_eq!(lines.len(), 3, "expected 3 lines");
		assert_eq!(lines[0].len(), 80);
		assert_eq!(lines[2].len(), 80);
		assert_eq!(lines[0], "/".repeat(80));
		assert_eq!(lines[2], "/".repeat(80));
		assert_eq!(lines[1].len(), 80, "middle line should be 80 chars");
		assert!(lines[1].starts_with("//"));
		assert!(lines[1].ends_with("//"));
		assert!(lines[1].contains("Banner text here."));
	}

	/// Verify correct banner for odd-padding text (extra space goes
	/// right).
	///
	/// `"Hello"` is 5 characters. Inner width is 76.
	/// Left = (76 − 5) / 2 = 35, right = 76 − 5 − 35 = 36.
	#[tokio::test]
	async fn banner_odd_padding()
	{
		let client = setup().await;
		let result = call_banner(&client, "Hello").await.unwrap();
		let text = first_text(&result);
		let lines: Vec<&str> = text.lines().collect();
		assert_eq!(lines.len(), 3);
		assert_eq!(lines[1].len(), 80);
		// Verify asymmetric padding: 35 spaces left, 36 right.
		let middle = lines[1];
		assert_eq!(&middle[..2], "//");
		assert_eq!(&middle[78..], "//");
		let inner = &middle[2..78];
		let left_spaces = inner.len() - inner.trim_start().len();
		let right_spaces = inner.len() - inner.trim_end().len();
		assert_eq!(left_spaces, 35, "expected 35 left spaces");
		assert_eq!(right_spaces, 36, "expected 36 right spaces");
	}

	/// Verify correct banner for short text (1 character).
	#[tokio::test]
	async fn banner_short_text()
	{
		let client = setup().await;
		let result = call_banner(&client, "X").await.unwrap();
		let text = first_text(&result);
		let lines: Vec<&str> = text.lines().collect();
		assert_eq!(lines.len(), 3);
		assert_eq!(lines[0].len(), 80);
		assert_eq!(lines[1].len(), 80);
		assert_eq!(lines[2].len(), 80);
		assert!(lines[1].contains("X"));
	}

	/// Verify that text longer than 74 characters returns an error.
	#[tokio::test]
	async fn banner_text_too_long()
	{
		let client = setup().await;
		let long_text = "x".repeat(75);
		let result = call_banner(&client, &long_text).await;
		assert!(
			result.is_err(),
			"text exceeding 74 chars should return an error"
		);
	}

	/// Verify that empty text produces a valid banner with all spaces
	/// in the middle.
	#[tokio::test]
	async fn banner_empty_text()
	{
		let client = setup().await;
		let result = call_banner(&client, "").await.unwrap();
		let text = first_text(&result);
		let lines: Vec<&str> = text.lines().collect();
		assert_eq!(lines.len(), 3);
		assert_eq!(lines[0], "/".repeat(80));
		assert_eq!(lines[2], "/".repeat(80));
		assert_eq!(lines[1].len(), 80);
		// Middle line should be // + 76 spaces + //.
		assert_eq!(lines[1], format!("//{spaces}//", spaces = " ".repeat(76)));
	}

	/// Verify the exact output for the spec example.
	#[tokio::test]
	async fn banner_spec_example()
	{
		let result = format_banner("Banner text here.").unwrap();
		assert_eq!(
			result,
			"////////////////////////////////////////////////////////////////////////////////\n\
			 //                             Banner text here.                              //\n\
			 ////////////////////////////////////////////////////////////////////////////////"
		);
	}

	/// Verify boundary: exactly 74 characters should succeed.
	#[tokio::test]
	async fn banner_max_length_text()
	{
		let text = "x".repeat(74);
		let result = format_banner(&text).unwrap();
		let lines: Vec<&str> = result.lines().collect();
		assert_eq!(lines.len(), 3);
		assert_eq!(lines[1].len(), 80);
		// 74 chars + 1 space left + 1 space right + 4 delimiters = 80.
		assert_eq!(lines[1], format!("// {text} //", text = "x".repeat(74)));
	}

	/// Verify the `format_banner` function directly for unit-level
	/// coverage.
	#[tokio::test]
	async fn format_banner_direct()
	{
		// Even: "Test" is 4 chars. Left = (76−4)/2 = 36, right = 36.
		let result = format_banner("Test").unwrap();
		let lines: Vec<&str> = result.lines().collect();
		let inner = &lines[1][2..78];
		assert_eq!(inner.trim(), "Test");
		let left = inner.len() - inner.trim_start().len();
		let right = inner.len() - inner.trim_end().len();
		assert_eq!(left, 36);
		assert_eq!(right, 36);
	}
}
