# banners-mcp

[![CI](https://github.com/toddATavail/banners-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/toddATavail/banners-mcp/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/banners-mcp.svg)](https://crates.io/crates/banners-mcp)
[![docs.rs](https://img.shields.io/docsrs/banners-mcp)](https://docs.rs/banners-mcp)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

An MCP server that generates perfectly centered 80-column Rust comment banners.

```
////////////////////////////////////////////////////////////////////////////////
//                             Banner text here.                              //
////////////////////////////////////////////////////////////////////////////////
```

## Banner specification

| Property | Value |
|----------|-------|
| Total width | 80 columns |
| Line 1 | 80 forward slashes |
| Line 2 | `//` + left padding + text + right padding + `//` |
| Line 3 | 80 forward slashes |
| Inner width | 76 characters (80 - 4 for `//` delimiters) |
| Left padding | `(76 - text_len) / 2` (integer division, floors) |
| Right padding | `76 - text_len - left` (gets extra space when odd) |
| Max text length | 74 characters (at least 1 space each side) |

## Installation

```sh
cargo install banners-mcp
```

Or build from source:

```sh
git clone https://github.com/toddATavail/banners-mcp.git
cd banners-mcp
cargo build --release
```

The binary will be at `target/release/banners-mcp`.

## Usage with Claude Code

```sh
claude mcp add banners-mcp -- banners-mcp
```

## Usage with other MCP clients

Add to your MCP client configuration:

```json
{
  "mcpServers": {
    "banners-mcp": {
      "command": "banners-mcp"
    }
  }
}
```

## MCP tool reference

### `banner`

Format text into a perfectly centered 80-column Rust comment banner.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `string` | Yes | The text to center. Must be 74 characters or fewer. |

**Returns:** A text content block containing the 3-line banner.

**Errors:** Returns an `InvalidParams` error if text exceeds 74 characters.

## Example

**Input:**

```json
{ "text": "Server definition." }
```

**Output:**

```
////////////////////////////////////////////////////////////////////////////////
//                             Server definition.                             //
////////////////////////////////////////////////////////////////////////////////
```

## License

MIT
