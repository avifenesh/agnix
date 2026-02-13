# MCP Protocol - Best Practices & Opinions

**Protocol Version**: 2025-11-25
**Last Updated**: 2026-02-13
**Nature**: Opinionated Recommendations (SHOULD/MAY/RECOMMENDED)

This document collects best practices, design recommendations, and opinionated guidance for implementing Model Context Protocol servers and clients. These are not hard requirements but represent community wisdom and design patterns.

---

## Table of Contents

1. [Naming Conventions](#naming-conventions)
2. [Schema Design Best Practices](#schema-design-best-practices)
3. [Description Writing Guidelines](#description-writing-guidelines)
4. [Error Message Design](#error-message-design)
5. [Performance Considerations](#performance-considerations)
6. [Tool Design Patterns](#tool-design-patterns)
7. [Resource Organization](#resource-organization)
8. [Prompt Engineering](#prompt-engineering)
9. [Security Best Practices](#security-best-practices)
10. [Testing and Debugging](#testing-and-debugging)
11. [User Experience Guidelines](#user-experience-guidelines)
12. [Architecture Patterns](#architecture-patterns)

---

## Naming Conventions

### Tool Names

**RECOMMENDED Patterns**:

```
✅ GOOD:
- get_weather (action_object pattern)
- search_flights (action_object pattern)
- calculate_distance (action_object pattern)
- send_email (action_object pattern)

❌ BAD:
- weather (too vague)
- calculate (incomplete)
- tool1 (meaningless)
- getWeather (inconsistent casing)
```

**Best Practices**:

1. **Use snake_case** → Consistent with JSON Schema conventions
2. **Start with action verb** → Makes purpose immediately clear (`get_`, `search_`, `create_`, `update_`, `delete_`)
3. **Include object/domain** → `weather_forecast` not just `forecast`
4. **Be specific not generic** → `search_flights` not `search`
5. **Avoid abbreviations** → `calculate_distance` not `calc_dist` (unless industry standard like `http_get`)

**Domain Prefixing** (for large servers):
```
✅ GOOD for multi-domain servers:
- database_query
- database_insert
- api_call
- api_authenticate
- file_read
- file_write
```

**Source**: Derived from examples in [Build Server Guide](https://modelcontextprotocol.io/docs/develop/build-server)

**Rationale**: Clear naming reduces cognitive load and makes tool discovery intuitive

---

### Resource URIs

**RECOMMENDED Patterns**:

```
✅ GOOD:
- file:///project/src/main.rs (specific, hierarchical)
- database://tables/users/schema (organized structure)
- api://github/repos/owner/name (clear namespace)
- calendar://events/2024-06-15 (date-based)

❌ BAD:
- res://1234 (opaque identifier)
- data://stuff (too vague)
- x://y (meaningless)
```

**Best Practices**:

1. **Use standard schemes when applicable** → `file://`, `https://`, `git://`
2. **Create hierarchical structures** → Easier to browse and understand
3. **Include context in path** → `/users/123/profile` not just `/123`
4. **Use readable identifiers** → `/projects/weather-server` not `/projects/a8f2d`
5. **Consistent path separators** → Always use `/` for hierarchy

**Custom Scheme Design**:
```
PATTERN: scheme://domain/collection/item/property

Examples:
- database://production/tables/users/schema
- cache://redis/keys/session:abc123
- queue://rabbitmq/exchange/orders/messages
```

**Source**: [Resources - Common URI Schemes](https://modelcontextprotocol.io/specification/2025-11-25/server/resources#common-uri-schemes)

**Rationale**: Well-structured URIs are self-documenting and enable pattern-based discovery

---

### Prompt Names

**RECOMMENDED Patterns**:

```
✅ GOOD:
- plan_vacation (action-oriented)
- review_code (clear purpose)
- analyze_data (specific task)
- draft_email (verb_noun pattern)

❌ BAD:
- vacation (unclear if it's planning, booking, or something else)
- code (too generic)
- prompt1 (meaningless)
```

**Best Practices**:

1. **Use action verbs** → `plan_`, `review_`, `analyze_`, `draft_`
2. **Describe outcome** → Name should indicate what user will get
3. **Group related prompts** → Use prefixes for categories (`email_draft`, `email_reply`, `email_forward`)
4. **Keep it concise** → 2-3 words maximum

---

## Schema Design Best Practices

### Input Schema Structure

**RECOMMENDED Pattern**:

```json
{
  "type": "object",
  "properties": {
    "required_param": {
      "type": "string",
      "description": "Clear description of what this parameter does",
      "minLength": 1
    },
    "optional_param": {
      "type": "number",
      "description": "Explain the purpose and valid range",
      "minimum": 0,
      "maximum": 100,
      "default": 50
    },
    "enum_param": {
      "type": "string",
      "enum": ["option1", "option2", "option3"],
      "description": "Use enums to constrain values",
      "default": "option1"
    }
  },
  "required": ["required_param"],
  "additionalProperties": false
}
```

**Best Practices**:

1. **Always include descriptions** → Help LLMs understand parameter purpose
2. **Use constraints liberally** → `minLength`, `maxLength`, `minimum`, `maximum`, `pattern`
3. **Provide defaults** → Make optional parameters easier to use
4. **Use enums for fixed sets** → Better than free-text when options are limited
5. **Set `additionalProperties: false`** → Catch typos and invalid parameters early
6. **Use consistent types** → Don't mix `string` and `number` for same concept
7. **Validate nested objects** → Fully specify structure of complex parameters

**Example: Well-Designed Search Tool**:

```json
{
  "name": "search_flights",
  "description": "Search for available flights between cities",
  "inputSchema": {
    "type": "object",
    "properties": {
      "origin": {
        "type": "string",
        "description": "IATA airport code for departure city (e.g., 'JFK', 'LAX')",
        "pattern": "^[A-Z]{3}$",
        "examples": ["JFK", "LAX", "ORD"]
      },
      "destination": {
        "type": "string",
        "description": "IATA airport code for arrival city",
        "pattern": "^[A-Z]{3}$"
      },
      "date": {
        "type": "string",
        "format": "date",
        "description": "Departure date in ISO 8601 format (YYYY-MM-DD)"
      },
      "passengers": {
        "type": "integer",
        "description": "Number of passengers",
        "minimum": 1,
        "maximum": 9,
        "default": 1
      },
      "class": {
        "type": "string",
        "enum": ["economy", "premium", "business", "first"],
        "description": "Cabin class preference",
        "default": "economy"
      }
    },
    "required": ["origin", "destination", "date"],
    "additionalProperties": false
  }
}
```

**Why This Is Good**:
- ✅ All parameters have detailed descriptions
- ✅ IATA codes validated with regex pattern
- ✅ Date format specified
- ✅ Passengers constrained to reasonable range
- ✅ Class limited to valid options with enum
- ✅ Sensible defaults provided
- ✅ Examples help LLM understand format
- ✅ `additionalProperties: false` catches errors

**Source**: Patterns from [Tools Specification](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)

**Rationale**: Rich schemas enable LLMs to use tools correctly without trial-and-error

---

### Output Schema Design

**RECOMMENDED when providing output schemas**:

```json
{
  "outputSchema": {
    "type": "object",
    "properties": {
      "temperature": {
        "type": "number",
        "description": "Temperature in celsius"
      },
      "conditions": {
        "type": "string",
        "description": "Weather conditions description"
      },
      "humidity": {
        "type": "number",
        "description": "Humidity percentage",
        "minimum": 0,
        "maximum": 100
      },
      "last_updated": {
        "type": "string",
        "format": "date-time",
        "description": "When data was last updated"
      }
    },
    "required": ["temperature", "conditions"]
  }
}
```

**Best Practices**:

1. **Include output schema when output is structured** → Helps clients validate and parse responses
2. **Match actual output structure** → Don't promise fields you won't return
3. **Document all fields** → Even if structure seems obvious
4. **Use consistent formatting** → Always ISO 8601 for dates, etc.
5. **Mark required fields** → Clients can depend on these always being present

**Source**: [Tools - Output Schema](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#output-schema)

---

## Description Writing Guidelines

### Tool Descriptions

**RECOMMENDED Pattern**:

```
TEMPLATE: [Verb] [object] [optional: context/constraints]

✅ GOOD Examples:
- "Search for available flights between two cities on a specific date"
- "Get current weather information including temperature, conditions, and forecast"
- "Send an email to one or more recipients with optional attachments"
- "Calculate the distance between two geographic coordinates using haversine formula"

❌ BAD Examples:
- "Gets weather" (too terse, no context)
- "This tool is used to search for flights that are available" (verbose, unnatural)
- "Flight searcher" (noun phrase, not descriptive)
```

**Best Practices**:

1. **Start with action verb** → Immediately conveys purpose
2. **Be specific about what it does** → Not just domain, but actual operation
3. **Include key constraints** → "between two cities", "with optional attachments"
4. **Keep it concise** → One clear sentence, ~15-25 words
5. **Avoid technical jargon** → Unless necessary for domain experts
6. **Write for LLM consumption** → Clear, unambiguous language

**Extended Description Pattern** (for complex tools):

```json
{
  "name": "search_flights",
  "description": "Search for available flights between two cities on a specific date",
  "title": "Flight Search",
  "inputSchema": {
    "type": "object",
    "description": "Provide origin and destination as 3-letter IATA codes (e.g., JFK, LAX). Results include prices, flight times, and available seats across multiple airlines.",
    "properties": { ... }
  }
}
```

**Note**: Use schema-level `description` for additional context about usage

---

### Parameter Descriptions

**RECOMMENDED Pattern**:

```
TEMPLATE: [What it is] [optional: format/constraints] [optional: examples]

✅ GOOD Examples:
- "City name or zip code (e.g., 'New York', '10001')"
- "IATA airport code for departure city (3 letters, e.g., 'JFK', 'LAX')"
- "Temperature units: 'celsius', 'fahrenheit', or 'kelvin'"
- "Budget amount in USD, must be positive number"

❌ BAD Examples:
- "location" (restates the parameter name)
- "The location parameter" (verbose)
- "string" (describes type, not meaning)
```

**Best Practices**:

1. **Explain the semantic meaning** → Not just the type
2. **Include format requirements** → Especially for strings
3. **Provide examples** → Helps LLM generate valid values
4. **Mention constraints** → "must be positive", "3 letters"
5. **Avoid redundancy with parameter name** → Don't just say "location" for `location` param

---

### Error Message Design

**RECOMMENDED Patterns**:

```
✅ GOOD Error Messages:
- "Invalid airport code 'XYZ'. Airport codes must be exactly 3 uppercase letters (e.g., 'JFK', 'LAX')."
- "Date '2024-13-45' is invalid. Use format YYYY-MM-DD (e.g., '2024-06-15')."
- "API rate limit exceeded. Please retry in 60 seconds."
- "Resource not found: file:///nonexistent.txt. Verify the file exists and you have read permissions."

❌ BAD Error Messages:
- "Error: 400" (no context)
- "Invalid input" (too generic)
- "XYZ not found" (unclear what XYZ refers to)
- "An error occurred" (useless)
```

**Best Practices**:

1. **State what went wrong** → Clear problem description
2. **Include the invalid value** → Echo back what was received
3. **Explain what's expected** → Format, constraints, examples
4. **Provide actionable guidance** → How to fix it
5. **Use consistent error format** → Makes parsing easier

**Structured Error Pattern**:

```json
{
  "isError": true,
  "content": [
    {
      "type": "text",
      "text": "Invalid parameter: 'temperature_unit' must be one of: ['celsius', 'fahrenheit', 'kelvin']. Received: 'centigrade'."
    }
  ]
}
```

**Source**: [Tools - Error Handling](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#error-handling)

**Rationale**: Good error messages reduce debugging time and improve LLM's ability to self-correct

---

## Performance Considerations

### Response Time Optimization

**RECOMMENDED Practices**:

1. **Set aggressive timeouts** → Default to 30-60 seconds for tool calls
2. **Use streaming for long operations** → Return partial results via SSE
3. **Implement caching** → Cache expensive computations, API responses
4. **Lazy load resources** → Don't fetch all resources in `resources/list`
5. **Paginate large result sets** → Use cursor-based pagination

**Example: Efficient Resource Listing**:

```json
{
  "method": "resources/list",
  "result": {
    "resources": [ /* first page */ ],
    "nextCursor": "eyJvZmZzZXQiOjEwMH0="
  }
}
```

**Source**: [Resources - Pagination](https://modelcontextprotocol.io/specification/2025-11-25/server/utilities/pagination)

---

### Memory Management

**RECOMMENDED Practices**:

1. **Clean up connections** → Close database connections, file handles
2. **Limit concurrent operations** → Use semaphores/pools to cap parallelism
3. **Stream large files** → Don't load entire files into memory
4. **Use generators/iterators** → For processing large datasets

**Example: Streaming File Content**:

```python
# ✅ GOOD: Stream large files
async def read_file_resource(uri):
    async with aiofiles.open(path, 'r') as f:
        async for line in f:
            yield line

# ❌ BAD: Load entire file
def read_file_resource(uri):
    with open(path, 'r') as f:
        return f.read()  # Could be gigabytes!
```

---

### Rate Limiting Best Practices

**RECOMMENDED Implementation**:

```python
from datetime import datetime, timedelta
from collections import defaultdict

class RateLimiter:
    def __init__(self, max_calls=10, window_seconds=60):
        self.max_calls = max_calls
        self.window = timedelta(seconds=window_seconds)
        self.calls = defaultdict(list)

    def check_limit(self, client_id):
        now = datetime.now()
        # Remove old calls outside window
        self.calls[client_id] = [
            t for t in self.calls[client_id]
            if now - t < self.window
        ]

        if len(self.calls[client_id]) >= self.max_calls:
            return False  # Rate limited

        self.calls[client_id].append(now)
        return True
```

**Best Practices**:

1. **Implement per-client limits** → Use session ID or connection ID
2. **Use sliding windows** → More fair than fixed windows
3. **Return 429 status** → Standard "Too Many Requests" code
4. **Include retry-after header** → Tell client when to retry
5. **Log rate limit hits** → Monitor for abuse patterns

---

## Tool Design Patterns

### Single Responsibility Principle

**RECOMMENDED**: Each tool should do one thing well

```
✅ GOOD: Separate tools for distinct operations
- search_flights (searches)
- book_flight (books)
- cancel_flight (cancels)
- get_flight_status (retrieves status)

❌ BAD: One tool does everything
- manage_flights (search, book, cancel, status all in one)
```

**Rationale**: Simpler tools are easier for LLMs to use correctly and compose together

---

### Composition Over Complexity

**RECOMMENDED**: Build complex workflows from simple tools

```
Example: Email with attachments

✅ GOOD: Compose simple tools
1. upload_file(path) → returns file_id
2. send_email(to, subject, body, attachment_ids=[file_id])

❌ BAD: One complex tool
- send_email_with_attachments(to, subject, body, file_paths=[...])
  → Requires file reading, encoding, email sending all in one
```

**Benefits**:
- Easier to test each piece
- More flexible (can reuse upload_file for other purposes)
- Clearer error handling
- Better separation of concerns

---

### Idempotency

**RECOMMENDED**: Make tools idempotent when possible

```python
# ✅ GOOD: Idempotent tool
def create_calendar_event(event_id, title, date):
    """Create event with specific ID. If exists, returns existing event."""
    existing = get_event(event_id)
    if existing:
        return existing
    return insert_event(event_id, title, date)

# ❌ BAD: Non-idempotent tool
def create_calendar_event(title, date):
    """Always creates new event, even if duplicate."""
    return insert_event(generate_id(), title, date)
```

**Why**: LLMs may retry operations; idempotency prevents duplicates

---

## Resource Organization

### Hierarchical Structure

**RECOMMENDED Pattern**:

```
GOOD: Organized hierarchy
database://production/
├── tables/
│   ├── users/
│   │   ├── schema
│   │   └── data
│   └── orders/
│       ├── schema
│       └── data
└── views/
    └── user_orders

BAD: Flat structure
database://users_schema
database://users_data
database://orders_schema
database://orders_data
database://user_orders_view
```

**Benefits**:
- Easier browsing and discovery
- Natural grouping of related resources
- Supports prefix-based filtering
- Matches mental models

---

### Resource Templates for Dynamic Content

**RECOMMENDED Use Cases**:

1. **Parameterized queries** → `database://queries/{query_name}`
2. **Date-based resources** → `logs://app/{date}/{level}`
3. **User-specific data** → `profiles://users/{user_id}`
4. **API endpoints** → `api://github/repos/{owner}/{repo}`

**Example: Well-Designed Template**:

```json
{
  "uriTemplate": "weather://forecast/{city}/{date}",
  "name": "weather-forecast",
  "title": "Weather Forecast",
  "description": "Get weather forecast for any city and date. City can be name or coordinates. Date must be within 14 days.",
  "mimeType": "application/json"
}
```

**Source**: [Resources - Resource Templates](https://modelcontextprotocol.io/specification/2025-11-25/server/resources#resource-templates)

---

### Annotations for Resources

**RECOMMENDED Usage**:

```json
{
  "uri": "file:///project/README.md",
  "name": "README.md",
  "annotations": {
    "audience": ["user"],
    "priority": 0.9,
    "lastModified": "2025-01-12T15:00:58Z"
  }
}
```

**Best Practices**:

1. **Use `audience`** → Guide clients on who should see this
   - `["user"]` → For human consumption
   - `["assistant"]` → For LLM context
   - `["user", "assistant"]` → Both

2. **Use `priority`** → Help clients decide what to include
   - `1.0` → Critical, always include
   - `0.5-0.9` → Important, include if space permits
   - `0.0-0.4` → Optional, include only if specifically requested

3. **Use `lastModified`** → Enable cache invalidation and freshness checks
   - Always ISO 8601 format
   - Include timezone (prefer UTC)

**Source**: [Resources - Annotations](https://modelcontextprotocol.io/specification/2025-11-25/server/resources#annotations)

---

## Prompt Engineering

### Argument Design

**RECOMMENDED Pattern**:

```json
{
  "name": "plan_vacation",
  "description": "Guide through vacation planning process",
  "arguments": [
    {
      "name": "destination",
      "description": "City or region to visit",
      "required": true
    },
    {
      "name": "duration",
      "description": "Number of days for the trip",
      "required": false
    },
    {
      "name": "budget",
      "description": "Maximum budget in USD",
      "required": false
    }
  ]
}
```

**Best Practices**:

1. **Keep arguments minimal** → Ask only for essential information
2. **Make most arguments optional** → Provide defaults or let LLM infer
3. **Provide clear descriptions** → Help users understand what to input
4. **Support flexible formats** → "New York", "NYC", "New York City" should all work
5. **Order by importance** → Required first, then most important optional ones

---

### Message Structure

**RECOMMENDED Pattern**:

```json
{
  "messages": [
    {
      "role": "user",
      "content": {
        "type": "text",
        "text": "You are a travel planning expert. Help the user plan a vacation to {destination} for {duration} days with a budget of ${budget}. Consider flights, accommodations, activities, and dining. Provide a day-by-day itinerary."
      }
    }
  ]
}
```

**Best Practices**:

1. **Set clear context** → Define the LLM's role
2. **Include all relevant parameters** → Reference prompt arguments
3. **Specify expected output format** → "Provide a day-by-day itinerary"
4. **Add constraints** → Budget limits, time restrictions, etc.
5. **Consider adding example interactions** → Few-shot prompting

---

## Security Best Practices

### Input Validation Defense-in-Depth

**RECOMMENDED Layers**:

```python
# Layer 1: Schema validation (automatic)
validate_against_json_schema(input, tool.inputSchema)

# Layer 2: Semantic validation
def validate_airport_code(code):
    if not re.match(r'^[A-Z]{3}$', code):
        raise ValueError("Invalid format")
    if code not in VALID_IATA_CODES:
        raise ValueError("Unknown airport")
    return code

# Layer 3: Business logic validation
def validate_date(date_str):
    date = parse_date(date_str)
    if date < datetime.now():
        raise ValueError("Date must be in future")
    if date > datetime.now() + timedelta(days=365):
        raise ValueError("Date too far in future")
    return date

# Layer 4: Authorization check
def check_permissions(user, resource):
    if not has_access(user, resource):
        raise PermissionError("Unauthorized")
```

**Source**: [Tools - Security Considerations](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#security-considerations)

---

### Sandboxing Tool Execution

**RECOMMENDED Approach**:

1. **Run in restricted environment** → Containers, VMs, or sandboxed processes
2. **Limit file system access** → Only allow access to specific directories
3. **Restrict network access** → Allowlist specific endpoints
4. **Set resource limits** → CPU, memory, time limits
5. **Drop privileges** → Run with minimal required permissions

**Example: Docker-based Sandboxing**:

```python
import docker

def execute_code_tool(code):
    client = docker.from_env()

    container = client.containers.run(
        'python:3.11-alpine',
        command=['python', '-c', code],
        mem_limit='256m',
        cpu_period=100000,
        cpu_quota=50000,  # 50% of one CPU
        network_disabled=True,
        read_only=True,
        remove=True,
        timeout=30
    )

    return container.decode('utf-8')
```

---

### Safe Resource URI Handling

**RECOMMENDED Pattern**:

```python
def validate_file_uri(uri):
    # Parse URI
    parsed = urlparse(uri)

    if parsed.scheme != 'file':
        raise ValueError("Only file:// URIs allowed")

    # Convert to absolute path
    path = os.path.abspath(parsed.path)

    # Check it's within allowed directory
    allowed_dir = '/allowed/base/path'
    if not path.startswith(allowed_dir):
        raise ValueError("Path outside allowed directory")

    # Check for path traversal
    if '..' in path:
        raise ValueError("Path traversal detected")

    # Resolve symlinks and verify again
    real_path = os.path.realpath(path)
    if not real_path.startswith(allowed_dir):
        raise ValueError("Symlink escape detected")

    return real_path
```

**Source**: [Resources - Security Considerations](https://modelcontextprotocol.io/specification/2025-11-25/server/resources#security-considerations)

---

### Audit Logging

**RECOMMENDED Information to Log**:

```python
{
    "timestamp": "2025-01-31T10:30:00Z",
    "event_type": "tool_execution",
    "session_id": "abc123...",
    "tool_name": "send_email",
    "input": {
        "to": "user@example.com",
        "subject": "Meeting reminder"
        # Sanitize sensitive data in logs
    },
    "result": "success",
    "duration_ms": 1250,
    "user_id": "user456",
    "ip_address": "192.168.1.100"
}
```

**Best Practices**:

1. **Log all tool executions** → Who, what, when, result
2. **Include session context** → Session ID, user ID
3. **Sanitize sensitive data** → Don't log passwords, tokens, etc.
4. **Enable audit trail** → For security reviews and compliance
5. **Set retention policies** → Balance storage vs. security needs

---

## Testing and Debugging

### MCP Inspector Usage

**RECOMMENDED Tool**: [MCP Inspector](https://github.com/modelcontextprotocol/inspector)

```bash
# Test your server during development
npx @modelcontextprotocol/inspector node path/to/server.js
npx @modelcontextprotocol/inspector python path/to/server.py
```

**Benefits**:
- Interactive testing of tools, resources, prompts
- Validates JSON-RPC message format
- Helps debug capability negotiation
- Shows exact request/response flow

**Source**: [MCP Inspector GitHub](https://github.com/modelcontextprotocol/inspector)

---

### Unit Testing Patterns

**RECOMMENDED Approach**:

```python
import pytest
from mcp_server import WeatherServer

@pytest.fixture
def server():
    return WeatherServer()

def test_get_weather_valid_input(server):
    result = server.call_tool("get_weather", {
        "location": "New York"
    })

    assert not result["isError"]
    assert "temperature" in result["content"][0]["text"]

def test_get_weather_invalid_location(server):
    result = server.call_tool("get_weather", {
        "location": ""
    })

    assert result["isError"]
    assert "invalid" in result["content"][0]["text"].lower()

def test_tool_schema_validation(server):
    tools = server.list_tools()
    weather_tool = [t for t in tools if t["name"] == "get_weather"][0]

    # Verify schema structure
    assert "inputSchema" in weather_tool
    assert weather_tool["inputSchema"]["type"] == "object"
    assert "location" in weather_tool["inputSchema"]["properties"]
```

---

### Integration Testing

**RECOMMENDED Pattern**:

```python
import asyncio
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

async def test_end_to_end():
    # Start server as subprocess
    server_params = StdioServerParameters(
        command="python",
        args=["server.py"]
    )

    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            # Initialize
            await session.initialize()

            # List tools
            tools = await session.list_tools()
            assert len(tools) > 0

            # Call tool
            result = await session.call_tool(
                "get_weather",
                {"location": "New York"}
            )
            assert result is not None
```

---

### Logging Best Practices

**RECOMMENDED Pattern**:

```python
import logging
import sys

# ✅ GOOD: Configure logging to stderr
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    stream=sys.stderr  # CRITICAL for stdio transport!
)

logger = logging.getLogger(__name__)

def call_tool(name, args):
    logger.info(f"Calling tool: {name} with args: {args}")
    try:
        result = execute_tool(name, args)
        logger.info(f"Tool {name} completed successfully")
        return result
    except Exception as e:
        logger.error(f"Tool {name} failed: {str(e)}", exc_info=True)
        raise
```

**Critical for stdio transport**: Never use `print()` in stdio servers!

**Source**: [Build Server - Logging](https://modelcontextprotocol.io/docs/develop/build-server)

---

## User Experience Guidelines

### Tool Invocation UX

**RECOMMENDED Patterns**:

1. **Before Execution**:
   - Show user which tool will be called
   - Display input parameters clearly
   - Require confirmation for destructive operations
   - Allow user to modify parameters

2. **During Execution**:
   - Show progress indicator for long-running operations
   - Enable cancellation
   - Display intermediate results if possible

3. **After Execution**:
   - Show clear success/failure indicator
   - Display results in readable format
   - Provide option to retry on failure
   - Log action for audit trail

**Example UI Flow**:

```
🔧 About to call: send_email
   To: team@example.com
   Subject: "Weekly update"
   Body: [Preview...]

   [Approve] [Modify] [Cancel]

⏳ Sending email...

✅ Email sent successfully at 10:30 AM
```

**Source**: [Tools - User Interaction Model](https://modelcontextprotocol.io/specification/2025-11-25/server/tools#user-interaction-model)

---

### Resource Selection UX

**RECOMMENDED Patterns**:

1. **Tree View** → Hierarchical file/folder browser
2. **Search Interface** → Full-text search with filters
3. **Smart Suggestions** → AI-powered relevance ranking
4. **Bulk Selection** → Multi-select with checkboxes
5. **Preview on Hover** → Show content snippet

**Example Resource Picker**:

```
📁 Resources
├─ 📁 Project Files
│  ├─ ☑ README.md (18 KB)
│  ├─ ☐ CONTRIBUTING.md (5 KB)
│  └─ ☑ src/main.py (42 KB)
├─ 📁 Documentation
│  └─ ☐ API.md (103 KB)
└─ 🔗 External
   └─ ☑ https://api.example.com/schema

[Select All] [Clear] [Add Selected]
```

**Source**: [Resources - User Interaction Model](https://modelcontextprotocol.io/specification/2025-11-25/server/resources#user-interaction-model)

---

### Prompt Discovery UX

**RECOMMENDED Patterns**:

1. **Slash Commands** → Type `/` to see available prompts
2. **Command Palette** → ⌘K/Ctrl+K to search prompts
3. **Context Menu** → Right-click for relevant prompts
4. **Quick Actions** → Toolbar buttons for common prompts
5. **Categories** → Group prompts by domain

**Example Slash Command**:

```
/ [Type to search prompts]

📝 /draft_email - Create a professional email
🔍 /analyze_data - Perform data analysis
✈️ /plan_vacation - Plan a trip itinerary
🐛 /debug_code - Debug code issues
```

**Source**: [Prompts - User Interaction Model](https://modelcontextprotocol.io/specification/2025-11-25/server/prompts#user-interaction-model)

---

## Architecture Patterns

### Multi-Server Composition

**RECOMMENDED Pattern**:

```
Application (MCP Host)
├─ MCP Client → Weather Server (tools: get_forecast, get_alerts)
├─ MCP Client → Calendar Server (resources: events, tools: create_event)
├─ MCP Client → Email Server (tools: send_email, search_inbox)
└─ MCP Client → Database Server (resources: schemas, tools: query)
```

**Best Practices**:

1. **Separate concerns** → One server per domain/service
2. **Avoid overlap** → Don't duplicate functionality across servers
3. **Use composition** → Combine simple servers for complex workflows
4. **Maintain independence** → Servers shouldn't depend on each other
5. **Version independently** → Each server can evolve at its own pace

**Source**: [Architecture Overview](https://modelcontextprotocol.io/docs/learn/architecture)

---

### State Management

**RECOMMENDED Approach**:

```python
# ✅ GOOD: Session-based state
class SessionState:
    def __init__(self, session_id):
        self.session_id = session_id
        self.context = {}
        self.created_at = datetime.now()

    def set_context(self, key, value):
        self.context[key] = value

    def get_context(self, key):
        return self.context.get(key)

# Store per-session state
sessions = {}

def handle_initialize(session_id, params):
    sessions[session_id] = SessionState(session_id)
    return initialize_response

# ❌ BAD: Global state
current_location = None  # Shared across all sessions!

def set_location(location):
    global current_location
    current_location = location  # Race conditions!
```

**Best Practices**:

1. **Isolate session state** → Use session IDs to separate client state
2. **Clean up on disconnect** → Remove session state when client disconnects
3. **Avoid global state** → Unless truly shared and thread-safe
4. **Consider persistence** → For long-running operations, persist state

---

## Summary of Key Recommendations

### Naming (SHOULD)
- ✅ Use `snake_case` for tool names
- ✅ Start tool names with action verbs
- ✅ Create hierarchical resource URIs
- ✅ Keep prompt names action-oriented

### Schema Design (SHOULD)
- ✅ Always include parameter descriptions
- ✅ Use JSON Schema constraints liberally
- ✅ Provide default values for optional parameters
- ✅ Set `additionalProperties: false`
- ✅ Include output schemas when output is structured

### Descriptions (SHOULD)
- ✅ Write clear, concise tool descriptions (15-25 words)
- ✅ Explain semantic meaning in parameter descriptions
- ✅ Provide examples in descriptions
- ✅ Write error messages that are actionable

### Performance (SHOULD)
- ✅ Set reasonable timeouts (30-60 seconds)
- ✅ Implement pagination for large result sets
- ✅ Use caching for expensive operations
- ✅ Stream large responses when possible

### Security (SHOULD)
- ✅ Implement defense-in-depth validation
- ✅ Sandbox tool execution
- ✅ Validate all resource URIs
- ✅ Log all operations for audit trail
- ✅ Sanitize sensitive data in logs

### Testing (SHOULD)
- ✅ Use MCP Inspector during development
- ✅ Write unit tests for all tools
- ✅ Implement integration tests
- ✅ Always log to stderr in stdio transport

### UX (SHOULD)
- ✅ Show tool inputs before execution
- ✅ Require confirmation for destructive operations
- ✅ Provide progress indicators
- ✅ Enable cancellation
- ✅ Make prompts discoverable via slash commands

---

## Document Sources

This document synthesizes best practices from:

1. [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)
2. [Build Server Guide](https://modelcontextprotocol.io/docs/develop/build-server)
3. [Build Client Guide](https://modelcontextprotocol.io/docs/develop/build-client)
4. [Server Concepts](https://modelcontextprotocol.io/docs/learn/server-concepts)
5. [Architecture Overview](https://modelcontextprotocol.io/docs/learn/architecture)
6. [Tools Specification](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)
7. [Resources Specification](https://modelcontextprotocol.io/specification/2025-11-25/server/resources)
8. [Prompts Specification](https://modelcontextprotocol.io/specification/2025-11-25/server/prompts)
9. [MCP Inspector](https://github.com/modelcontextprotocol/inspector)
10. Community implementations and examples

**Revision History**:
- 2026-02-13: Updated all protocol version references from 2025-06-18 to 2025-11-25
- 2026-01-31: Initial best practices compilation from protocol version 2025-06-18

---

**End of MCP Best Practices Document**
