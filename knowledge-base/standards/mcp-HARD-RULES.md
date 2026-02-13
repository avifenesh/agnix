# MCP Protocol - Hard Rules (Non-Negotiable)

**Protocol Version**: 2025-11-25
**Last Updated**: 2026-02-13
**RFC Compliance**: BCP 14 [RFC2119] [RFC8174]

This document extracts the absolute, non-negotiable requirements from the Model Context Protocol specification. These are the MUST/MUST NOT rules that determine protocol compliance.

---

## Table of Contents

1. [RFC Language Interpretation](#rfc-language-interpretation)
2. [Protocol Architecture](#protocol-architecture)
3. [Lifecycle Management](#lifecycle-management)
4. [JSON-RPC 2.0 Compliance](#json-rpc-20-compliance)
5. [Tool Definition Schema](#tool-definition-schema)
6. [Resource Definition Schema](#resource-definition-schema)
7. [Prompt Definition Schema](#prompt-definition-schema)
8. [Transport Layer Requirements](#transport-layer-requirements)
9. [Security Requirements](#security-requirements)
10. [Error Handling](#error-handling)
11. [Protocol Anti-Patterns](#protocol-anti-patterns)

---

## RFC Language Interpretation

**Source**: [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in MCP are to be interpreted as described in BCP 14 [RFC2119] [RFC8174] when, and only when, they appear in all capitals.

**Rule**: Use only capitalized RFC 2119 keywords for normative statements.

---

## Protocol Architecture

### Core Participants

**MUST Requirements**:

1. **MCP Host** → MUST coordinate one or more MCP clients
2. **MCP Client** → MUST maintain a dedicated connection to exactly one MCP server
3. **MCP Server** → MUST expose context through tools, resources, and/or prompts

**Source**: [Architecture Overview](https://modelcontextprotocol.io/docs/learn/architecture)

### Layer Structure

**MUST Requirements**:

1. **Data Layer** → MUST implement JSON-RPC 2.0 protocol
2. **Transport Layer** → MUST provide bidirectional message exchange
3. Clients → MUST support stdio transport whenever possible

**Source**: [Architecture - Layers](https://modelcontextprotocol.io/docs/learn/architecture#layers)

---

## Lifecycle Management

### Initialization Phase

**Source**: [Lifecycle - Initialization](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)

#### Client Requirements (MUST):

1. **First Interaction** → Initialization MUST be the first interaction between client and server
2. **Initialize Request** → Client MUST send `initialize` request containing:
   - `protocolVersion` (string)
   - `capabilities` (object)
   - `clientInfo` (object with `name` and `version`)

3. **Initialized Notification** → After successful initialization, client MUST send `initialized` notification

4. **Pre-initialization** → Client SHOULD NOT send requests other than pings before server responds to `initialize`

**Example Initialize Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "roots": { "listChanged": true },
      "sampling": {},
      "elicitation": {}
    },
    "clientInfo": {
      "name": "ExampleClient",
      "version": "1.0.0"
    }
  }
}
```

#### Server Requirements (MUST):

1. **Response** → Server MUST respond with:
   - `protocolVersion` (negotiated version)
   - `capabilities` (object)
   - `serverInfo` (object with `name` and `version`)

2. **Pre-initialization** → Server SHOULD NOT send requests other than pings and logging before receiving `initialized` notification

**Example Initialize Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "tools": { "listChanged": true },
      "resources": { "subscribe": true, "listChanged": true }
    },
    "serverInfo": {
      "name": "ExampleServer",
      "version": "1.0.0"
    }
  }
}
```

### Version Negotiation

**MUST Requirements**:

1. Client → MUST send latest protocol version it supports in `initialize` request
2. Server → If it supports requested version, MUST respond with same version
3. Server → If it doesn't support requested version, MUST respond with another version it supports (SHOULD be latest)
4. Client → If it doesn't support server's version, SHOULD disconnect
5. HTTP Transport → Client MUST include `MCP-Protocol-Version: <protocol-version>` header on all subsequent HTTP requests

**Source**: [Lifecycle - Version Negotiation](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle#version-negotiation)

**Why**: Prevents communication errors from incompatible protocol versions

### Capability Negotiation

**MUST Requirements**:

1. Both parties MUST respect negotiated protocol version
2. Both parties MUST only use capabilities successfully negotiated during initialization
3. Servers declaring capabilities MUST implement them correctly

**Capability Table**:

| Category | Capability | MUST Implement If Declared |
|----------|------------|---------------------------|
| Server | `tools` | `tools/list`, `tools/call` methods |
| Server | `resources` | `resources/list`, `resources/read` methods |
| Server | `prompts` | `prompts/list`, `prompts/get` methods |
| Server | `logging` | Accept log messages |
| Client | `roots` | Provide filesystem roots |
| Client | `sampling` | Handle LLM sampling requests |
| Client | `elicitation` | Handle user input requests |

**Source**: [Lifecycle - Capability Negotiation](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle#capability-negotiation)

---

## JSON-RPC 2.0 Compliance

**Source**: [MCP Specification - Overview](https://modelcontextprotocol.io/specification/2025-11-25)

### Message Format

**MUST Requirements**:

1. All messages → MUST use JSON-RPC 2.0 format
2. All messages → MUST be UTF-8 encoded
3. Requests → MUST include:
   - `jsonrpc`: "2.0" (string, literal)
   - `id`: unique identifier (string or number)
   - `method`: method name (string)
   - `params`: parameters (object, optional)

4. Responses → MUST include:
   - `jsonrpc`: "2.0" (string, literal)
   - `id`: matching request id
   - Either `result` (any) OR `error` (object), not both

5. Notifications → MUST include:
   - `jsonrpc`: "2.0" (string, literal)
   - `method`: method name (string)
   - `params`: parameters (object, optional)
   - MUST NOT include `id` field

**Example Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "search",
    "arguments": { "query": "test" }
  }
}
```

**Example Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      { "type": "text", "text": "Results..." }
    ]
  }
}
```

**Example Notification**:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/tools/list_changed"
}
```

**Why**: JSON-RPC 2.0 is the foundational protocol; violations break interoperability

---

## Tool Definition Schema

**Source**: [Tools Specification](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)

### Tool Object Structure

**MUST Requirements**:

1. **name** (string, required) → MUST be unique within server
2. **description** (string, required) → MUST describe functionality
3. **inputSchema** (object, required) → MUST be valid JSON Schema
4. **title** (string, optional) → Human-readable display name
5. **outputSchema** (object, optional) → If provided, MUST be valid JSON Schema
6. **annotations** (object, optional) → Metadata about tool behavior

**Example Tool Definition**:
```json
{
  "name": "get_weather",
  "title": "Weather Information Provider",
  "description": "Get current weather information for a location",
  "inputSchema": {
    "type": "object",
    "properties": {
      "location": {
        "type": "string",
        "description": "City name or zip code"
      }
    },
    "required": ["location"]
  }
}
```

### Tool Capabilities Declaration

**MUST Requirements**:

1. Servers supporting tools → MUST declare `tools` capability during initialization
2. Declaration → MUST include `listChanged` boolean if server will send notifications

**Example Capability Declaration**:
```json
{
  "capabilities": {
    "tools": {
      "listChanged": true
    }
  }
}
```

**Source**: [Tools - Capabilities](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#capabilities)

### Tool Protocol Methods

**MUST Requirements**:

1. **tools/list** → Servers MUST respond with array of tool definitions
2. **tools/call** → Servers MUST validate input against `inputSchema`
3. **tools/call** → Servers with `outputSchema` MUST return conforming structured results
4. **notifications/tools/list_changed** → Servers with `listChanged: true` SHOULD send when tools change

**Example tools/list Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "tools": [
      {
        "name": "get_weather",
        "description": "Get weather data",
        "inputSchema": { "type": "object", "properties": { "location": { "type": "string" } }, "required": ["location"] }
      }
    ]
  }
}
```

**Example tools/call Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "get_weather",
    "arguments": {
      "location": "New York"
    }
  }
}
```

### Tool Security Requirements

**MUST Requirements (Trust & Safety)**:

1. Servers → MUST validate all tool inputs against inputSchema
2. Servers → MUST implement proper access controls
3. Servers → MUST sanitize tool outputs
4. Clients → MUST consider tool annotations untrusted unless from trusted servers
5. Applications → SHOULD provide UI showing which tools are exposed
6. Applications → SHOULD insert visual indicators when tools are invoked
7. Applications → SHOULD present confirmation prompts for operations

**Source**: [Tools - Security Considerations](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#security-considerations)

**Why**: Tools execute arbitrary code and can modify external systems

---

## Resource Definition Schema

**Source**: [Resources Specification](https://modelcontextprotocol.io/specification/2025-11-25/server/resources)

### Resource Object Structure

**MUST Requirements**:

1. **uri** (string, required) → MUST be unique identifier (RFC3986 compliant)
2. **name** (string, required) → MUST identify the resource
3. **title** (string, optional) → Human-readable display name
4. **description** (string, optional) → Resource description
5. **mimeType** (string, optional) → MIME type of content
6. **size** (number, optional) → Size in bytes
7. **annotations** (object, optional) → Metadata (audience, priority, lastModified)

**Example Resource Definition**:
```json
{
  "uri": "file:///project/src/main.rs",
  "name": "main.rs",
  "title": "Rust Application Main File",
  "description": "Primary application entry point",
  "mimeType": "text/x-rust"
}
```

### Resource Content Types

**MUST Requirements**:

1. Text content → MUST include `uri`, `mimeType`, `text` fields
2. Binary content → MUST include `uri`, `mimeType`, `blob` (base64-encoded) fields
3. Binary data → MUST be properly encoded as base64

**Example Text Content**:
```json
{
  "uri": "file:///example.txt",
  "mimeType": "text/plain",
  "text": "Resource content"
}
```

**Example Binary Content**:
```json
{
  "uri": "file:///example.png",
  "mimeType": "image/png",
  "blob": "base64-encoded-data"
}
```

### Resource URI Schemes

**MUST Requirements**:

1. **https://** → Servers SHOULD use only when client can fetch directly from web
2. **file://** → MAY use XDG MIME types for non-regular files (e.g., `inode/directory`)
3. **Custom URIs** → MUST be RFC3986 compliant

**Source**: [Resources - Common URI Schemes](https://modelcontextprotocol.io/specification/2025-11-25/server/resources#common-uri-schemes)

### Resource Capabilities Declaration

**MUST Requirements**:

1. Servers supporting resources → MUST declare `resources` capability
2. Optional features:
   - `subscribe` (boolean) → If true, MUST support subscription to resource changes
   - `listChanged` (boolean) → If true, MUST send notifications when resource list changes

**Example Capability Declaration**:
```json
{
  "capabilities": {
    "resources": {
      "subscribe": true,
      "listChanged": true
    }
  }
}
```

### Resource Protocol Methods

**MUST Requirements**:

1. **resources/list** → Servers MUST respond with array of resource definitions
2. **resources/read** → Servers MUST return content with proper encoding
3. **resources/subscribe** → Servers with `subscribe: true` MUST accept subscriptions
4. **notifications/resources/updated** → Servers with `subscribe: true` MUST notify on changes
5. **notifications/resources/list_changed** → Servers with `listChanged: true` SHOULD send when list changes

**Example resources/read Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "contents": [
      {
        "uri": "file:///project/src/main.rs",
        "mimeType": "text/x-rust",
        "text": "fn main() {\n    println!(\"Hello world!\");\n}"
      }
    ]
  }
}
```

### Resource Security Requirements

**MUST Requirements**:

1. Servers → MUST validate all resource URIs
2. Access controls → SHOULD be implemented for sensitive resources
3. Binary data → MUST be properly encoded
4. Resource permissions → SHOULD be checked before operations

**Source**: [Resources - Security Considerations](https://modelcontextprotocol.io/specification/2025-11-25/server/resources#security-considerations)

---

## Prompt Definition Schema

**Source**: [Prompts Specification](https://modelcontextprotocol.io/specification/2025-11-25/server/prompts)

### Prompt Object Structure

**MUST Requirements**:

1. **name** (string, required) → MUST be unique identifier
2. **description** (string, optional) → Human-readable description
3. **title** (string, optional) → Display name
4. **arguments** (array, optional) → List of argument definitions

**Argument Structure**:
- `name` (string, required)
- `description` (string, optional)
- `required` (boolean, optional)

**Example Prompt Definition**:
```json
{
  "name": "code_review",
  "title": "Request Code Review",
  "description": "Asks the LLM to analyze code quality and suggest improvements",
  "arguments": [
    {
      "name": "code",
      "description": "The code to review",
      "required": true
    }
  ]
}
```

### Prompt Message Structure

**MUST Requirements**:

1. Messages → MUST include `role` field ("user" or "assistant")
2. Messages → MUST include `content` field with one of:
   - Text content: `{ "type": "text", "text": "..." }`
   - Image content: `{ "type": "image", "data": "base64...", "mimeType": "image/png" }`
   - Audio content: `{ "type": "audio", "data": "base64...", "mimeType": "audio/wav" }`
   - Embedded resource: `{ "type": "resource", "resource": {...} }`

**Example Prompt Message**:
```json
{
  "role": "user",
  "content": {
    "type": "text",
    "text": "Please review this Python code:\ndef hello():\n    print('world')"
  }
}
```

### Prompt Capabilities Declaration

**MUST Requirements**:

1. Servers supporting prompts → MUST declare `prompts` capability during initialization
2. Declaration → MUST include `listChanged` boolean if server will send notifications

**Example Capability Declaration**:
```json
{
  "capabilities": {
    "prompts": {
      "listChanged": true
    }
  }
}
```

### Prompt Protocol Methods

**MUST Requirements**:

1. **prompts/list** → Servers MUST respond with array of prompt definitions
2. **prompts/get** → Servers MUST validate arguments and return prompt content
3. **notifications/prompts/list_changed** → Servers with `listChanged: true` SHOULD send when prompts change

**Example prompts/get Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "prompts/get",
  "params": {
    "name": "code_review",
    "arguments": {
      "code": "def hello():\n    print('world')"
    }
  }
}
```

### Prompt Security Requirements

**MUST Requirements**:

1. Implementations → MUST carefully validate all prompt inputs
2. Implementations → MUST prevent injection attacks
3. Implementations → MUST prevent unauthorized resource access

**Source**: [Prompts - Security](https://modelcontextprotocol.io/specification/2025-11-25/server/prompts#security)

---

## Transport Layer Requirements

**Source**: [Transports Specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)

### General Transport Requirements

**MUST Requirements**:

1. All messages → MUST be UTF-8 encoded
2. All messages → MUST use JSON-RPC 2.0 format
3. Transport → MUST support bidirectional message exchange

### stdio Transport

**MUST Requirements**:

1. Server → MUST read JSON-RPC messages from stdin
2. Server → MUST write JSON-RPC messages to stdout
3. Messages → MUST be delimited by newlines
4. Messages → MUST NOT contain embedded newlines
5. Server → MUST NOT write non-MCP content to stdout
6. Client → MUST NOT write non-MCP content to server's stdin
7. Server → MAY write UTF-8 logs to stderr
8. Client → MAY capture, forward, or ignore stderr logs

**Why**: Writing non-JSON-RPC content to stdout/stdin corrupts the message stream

**Shutdown Process**:
1. Client → SHOULD close input stream to server process
2. Client → SHOULD wait for server exit
3. Client → MAY send SIGTERM if server doesn't exit
4. Client → MAY send SIGKILL after SIGTERM timeout

**Anti-Pattern Example** (NEVER DO THIS):
```python
# ❌ BAD - Corrupts stdio transport
print("Processing request")  # Writes to stdout!
console.log("Server started")  # Writes to stdout!

# ✅ GOOD - Use stderr or logging
import logging
logging.info("Processing request")  # Writes to stderr
console.error("Server started")  # Writes to stderr
```

**Source**: [Build Server - Logging](https://modelcontextprotocol.io/docs/develop/build-server)

### HTTP Transport (Streamable HTTP)

**MUST Requirements**:

#### Client-to-Server Messages:

1. Client → MUST use HTTP POST to send JSON-RPC messages
2. Client → MUST include `Accept: application/json, text/event-stream` header
3. POST body → MUST be single JSON-RPC request, notification, or response
4. Client → MUST include `MCP-Protocol-Version: <version>` header on all subsequent requests
5. Server → For responses/notifications, MUST return HTTP 202 Accepted (no body) if accepted
6. Server → For requests, MUST return either `Content-Type: text/event-stream` (SSE) OR `Content-Type: application/json`

#### SSE Streams:

1. SSE stream → SHOULD eventually include JSON-RPC response for the request
2. Server → MAY send requests/notifications before response
3. Server → SHOULD NOT close SSE before sending response (unless session expires)
4. Server → SHOULD close SSE after sending response
5. Server → MAY attach event IDs for resumability (MUST be globally unique per session)
6. Server → MUST NOT replay messages from different streams

#### GET Endpoint:

1. Client → MAY issue HTTP GET to open SSE stream
2. Client → MUST include `Accept: text/event-stream` header
3. Server → MUST return `Content-Type: text/event-stream` OR HTTP 405 Method Not Allowed
4. Server → MAY send requests/notifications on stream
5. Server → MUST NOT send responses on GET stream (unless resuming)

#### Session Management:

1. Server → MAY assign session ID in `Mcp-Session-Id` header on initialize response
2. Session ID → SHOULD be globally unique and cryptographically secure
3. Session ID → MUST only contain visible ASCII (0x21 to 0x7E)
4. Client → MUST include `Mcp-Session-Id` header on all subsequent requests if provided
5. Server → SHOULD respond with HTTP 400 Bad Request to requests without required session ID
6. Server → MAY terminate session, then MUST return HTTP 404 Not Found for that session
7. Client → MUST start new session (send new initialize) when receiving HTTP 404 with session ID
8. Client → SHOULD send HTTP DELETE with `Mcp-Session-Id` to terminate session

#### Security:

1. Servers → MUST validate `Origin` header to prevent DNS rebinding attacks
2. Local servers → SHOULD bind only to localhost (127.0.0.1)
3. Servers → SHOULD implement proper authentication

**Source**: [Transports - Streamable HTTP](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports#streamable-http)

**Why**: Prevents DNS rebinding attacks and ensures secure connections

---

## Security Requirements

**Source**: Multiple sections of specification

### User Consent and Control (MUST)

**Requirements**:

1. Users → MUST explicitly consent to data access and operations
2. Users → MUST retain control over what data is shared
3. Users → MUST retain control over what actions are taken
4. Implementors → SHOULD provide clear UIs for reviewing/authorizing activities

**Source**: [Security Principles](https://modelcontextprotocol.io/specification/2025-11-25#security-and-trust--safety)

**Why**: Users must understand and control AI actions

### Data Privacy (MUST)

**Requirements**:

1. Hosts → MUST obtain explicit user consent before exposing user data to servers
2. Hosts → MUST NOT transmit resource data elsewhere without user consent
3. User data → SHOULD be protected with appropriate access controls

**Source**: [Security Principles - Data Privacy](https://modelcontextprotocol.io/specification/2025-11-25#security-and-trust--safety)

### Tool Safety (MUST)

**Requirements**:

1. Tools → MUST be treated with appropriate caution (arbitrary code execution)
2. Tool descriptions/annotations → MUST be considered untrusted unless from trusted server
3. Hosts → MUST obtain explicit user consent before invoking any tool
4. Users → SHOULD understand what each tool does before authorizing

**Source**: [Security Principles - Tool Safety](https://modelcontextprotocol.io/specification/2025-11-25#security-and-trust--safety)

**Why**: Tool descriptions could be malicious or misleading

### Input Validation (MUST)

**Requirements**:

1. Servers → MUST validate all tool inputs against inputSchema
2. Servers → MUST validate all resource URIs
3. Servers → MUST validate all prompt inputs
4. Servers → MUST sanitize tool outputs
5. Clients → MUST validate tool results before passing to LLM

**Source**: Various security sections

### Rate Limiting and Resource Protection (SHOULD)

**Requirements**:

1. Servers → MUST rate limit tool invocations
2. Clients → SHOULD implement timeouts for tool calls
3. Clients → SHOULD log tool usage for audit purposes
4. Implementations → SHOULD establish timeouts for all sent requests

**Source**: [Tools - Security](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#security-considerations), [Lifecycle - Timeouts](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle#timeouts)

---

## Error Handling

**Source**: [Tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools), [Resources](https://modelcontextprotocol.io/specification/2025-11-25/server/resources), [Prompts](https://modelcontextprotocol.io/specification/2025-11-25/server/prompts)

### Standard Error Codes

**MUST Use These JSON-RPC Error Codes**:

| Code | Meaning | Usage |
|------|---------|-------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid request | Not valid JSON-RPC 2.0 |
| -32601 | Method not found | Unknown method |
| -32602 | Invalid params | Invalid parameters (e.g., unknown tool/resource/prompt, missing required arguments) |
| -32603 | Internal error | Server-side errors |
| -32002 | Resource not found | Specific to resources |

**Example Protocol Error**:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "error": {
    "code": -32602,
    "message": "Unknown tool: invalid_tool_name"
  }
}
```

### Tool Execution Errors

**MUST Requirements**:

1. Tool execution errors → MUST be reported in tool result with `isError: true`
2. Error result → MUST include descriptive content

**Example Tool Execution Error**:
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Failed to fetch weather data: API rate limit exceeded"
      }
    ],
    "isError": true
  }
}
```

**Why**: Distinguishes protocol errors from business logic errors

### Resource Errors

**SHOULD Use Standard Codes**:

- Resource not found: `-32002`
- Internal errors: `-32603`

**Example Resource Error**:
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "error": {
    "code": -32002,
    "message": "Resource not found",
    "data": {
      "uri": "file:///nonexistent.txt"
    }
  }
}
```

### Prompt Errors

**SHOULD Use Standard Codes**:

- Invalid prompt name: `-32602`
- Missing required arguments: `-32602`
- Internal errors: `-32603`

### Initialization Errors

**Example Version Mismatch Error**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Unsupported protocol version",
    "data": {
      "supported": ["2024-11-05"],
      "requested": "1.0.0"
    }
  }
}
```

---

## Protocol Anti-Patterns

These patterns VIOLATE the MCP protocol and MUST be avoided.

### 1. Writing to stdout in stdio Transport

**Anti-Pattern**:
```python
# ❌ NEVER DO THIS
print("Debug message")  # Corrupts JSON-RPC stream
console.log("Status")   # Corrupts JSON-RPC stream
fmt.Println("Info")     # Corrupts JSON-RPC stream
```

**Why Breaks Protocol**: Inserts non-JSON-RPC content into message stream, causing parse errors

**Correct Pattern**:
```python
# ✅ CORRECT
import logging
logging.info("Debug message")  # Writes to stderr
console.error("Status")        # Writes to stderr
```

**Source**: [Logging in MCP Servers](https://modelcontextprotocol.io/docs/develop/build-server)

---

### 2. Sending Requests Before Initialization

**Anti-Pattern**:
```json
// ❌ Client sends tools/list before initialize
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list"
}
```

**Why Breaks Protocol**: Capabilities haven't been negotiated; server doesn't know what client supports

**Correct Pattern**:
```json
// ✅ CORRECT: Initialize first
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": { "protocolVersion": "2025-11-25", ... }
}
// Wait for response, send initialized notification, then send tools/list
```

**Source**: [Lifecycle - Initialization](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)

---

### 3. Missing Required Fields

**Anti-Pattern**:
```json
// ❌ Tool definition missing required fields
{
  "name": "search"
  // Missing: description, inputSchema
}
```

**Why Breaks Protocol**: Violates schema requirements; clients can't understand tool

**Correct Pattern**:
```json
// ✅ CORRECT
{
  "name": "search",
  "description": "Search for information",
  "inputSchema": {
    "type": "object",
    "properties": { "query": { "type": "string" } },
    "required": ["query"]
  }
}
```

---

### 4. Invalid JSON Schema

**Anti-Pattern**:
```json
// ❌ Invalid JSON Schema in inputSchema
{
  "name": "search",
  "description": "Search",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": "string"  // Wrong! Should be {"type": "string"}
    }
  }
}
```

**Why Breaks Protocol**: JSON Schema validation fails; clients can't validate inputs

**Correct Pattern**:
```json
// ✅ CORRECT
{
  "name": "search",
  "description": "Search",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": { "type": "string" }
    }
  }
}
```

---

### 5. Announcing Capabilities Without Implementing Them

**Anti-Pattern**:
```json
// ❌ Server declares tools capability but doesn't implement tools/list
{
  "capabilities": {
    "tools": { "listChanged": true }
  }
}
// Then returns error for tools/list request
```

**Why Breaks Protocol**: Client expects advertised functionality; breaks contract

**Correct Pattern**: Only declare capabilities you actually implement

---

### 6. Not Validating Input Against Schema

**Anti-Pattern**:
```python
# ❌ Server doesn't validate tool input
def call_tool(name, arguments):
    # Directly uses arguments without validation
    return execute(arguments)
```

**Why Breaks Protocol**: Security risk; can cause runtime errors; violates MUST requirement

**Correct Pattern**:
```python
# ✅ CORRECT
def call_tool(name, arguments):
    tool = get_tool(name)
    validate_against_schema(arguments, tool.inputSchema)
    return execute(arguments)
```

**Source**: [Tools - Security](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#security-considerations)

---

### 7. Mixing Notifications with IDs

**Anti-Pattern**:
```json
// ❌ Notification includes id field
{
  "jsonrpc": "2.0",
  "id": 999,
  "method": "notifications/tools/list_changed"
}
```

**Why Breaks Protocol**: Notifications MUST NOT have `id`; this is a JSON-RPC requirement

**Correct Pattern**:
```json
// ✅ CORRECT
{
  "jsonrpc": "2.0",
  "method": "notifications/tools/list_changed"
}
```

---

### 8. Not Sending initialized Notification

**Anti-Pattern**:
```javascript
// ❌ Client skips initialized notification
await client.initialize();
// Immediately starts sending requests without initialized notification
await client.listTools();
```

**Why Breaks Protocol**: Server may not be ready; violates initialization handshake

**Correct Pattern**:
```javascript
// ✅ CORRECT
await client.initialize();
await client.sendInitializedNotification();
await client.listTools();
```

---

### 9. Returning Both result and error

**Anti-Pattern**:
```json
// ❌ Response has both result and error
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { "data": "..." },
  "error": { "code": -32603, "message": "Warning" }
}
```

**Why Breaks Protocol**: JSON-RPC 2.0 requires exactly one of `result` OR `error`

**Correct Pattern**:
```json
// ✅ CORRECT: Either result...
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { "data": "..." }
}

// ...or error
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": { "code": -32603, "message": "Error" }
}
```

---

### 10. Incorrect Protocol Version Header

**Anti-Pattern**:
```http
POST /mcp HTTP/1.1
// ❌ Missing or wrong protocol version header
Content-Type: application/json
```

**Why Breaks Protocol**: Server can't identify protocol version for HTTP transport

**Correct Pattern**:
```http
POST /mcp HTTP/1.1
MCP-Protocol-Version: 2025-11-25
Content-Type: application/json
```

**Source**: [Transports - Protocol Version Header](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports#protocol-version-header)

---

## Summary of Critical MUST Requirements

### Initialization
- ✅ Client MUST send `initialize` as first interaction
- ✅ Client MUST send `initialized` notification after initialization
- ✅ Both parties MUST respect negotiated capabilities

### JSON-RPC
- ✅ All messages MUST use JSON-RPC 2.0 format
- ✅ Messages MUST be UTF-8 encoded
- ✅ Responses MUST have either `result` OR `error`, not both
- ✅ Notifications MUST NOT have `id` field

### Tools
- ✅ Tool definitions MUST include `name`, `description`, `inputSchema`
- ✅ `inputSchema` MUST be valid JSON Schema
- ✅ Servers MUST validate inputs against schema
- ✅ Servers with `outputSchema` MUST return conforming results

### Resources
- ✅ Resource URIs MUST be RFC3986 compliant
- ✅ Resource URIs MUST be unique
- ✅ Binary data MUST be base64 encoded
- ✅ Servers MUST validate all resource URIs

### Prompts
- ✅ Prompt names MUST be unique
- ✅ Messages MUST include `role` and `content`
- ✅ Implementations MUST validate all inputs

### Transport
- ✅ stdio: MUST NOT write non-JSON-RPC to stdout/stdin
- ✅ stdio: Messages MUST be newline-delimited
- ✅ HTTP: Client MUST include `MCP-Protocol-Version` header
- ✅ HTTP: Servers MUST validate `Origin` header

### Security
- ✅ Servers MUST validate all inputs
- ✅ Hosts MUST obtain user consent for data access
- ✅ Tool annotations MUST be considered untrusted
- ✅ Servers MUST sanitize outputs

---

## Document Sources

This document synthesizes hard requirements from:

1. [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)
2. [Tools Specification](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)
3. [Resources Specification](https://modelcontextprotocol.io/specification/2025-11-25/server/resources)
4. [Prompts Specification](https://modelcontextprotocol.io/specification/2025-11-25/server/prompts)
5. [Lifecycle Management](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)
6. [Transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)
7. [Architecture Overview](https://modelcontextprotocol.io/docs/learn/architecture)
8. [Build Server Guide](https://modelcontextprotocol.io/docs/develop/build-server)
9. [Build Client Guide](https://modelcontextprotocol.io/docs/develop/build-client)
10. [Server Concepts](https://modelcontextprotocol.io/docs/learn/server-concepts)
11. [GitHub Specification Repository](https://github.com/modelcontextprotocol/specification)

**Revision History**:
- 2026-02-13: Updated all protocol version references from 2025-06-18 to 2025-11-25
- 2026-01-31: Initial comprehensive hard rules extraction from protocol version 2025-06-18

---

**End of MCP Hard Rules Document**
