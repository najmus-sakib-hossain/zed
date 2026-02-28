
# DCP Python Client

A native Python SDK for the Development Context Protocol (DCP), providing async/await support for all MCP operations.

## Installation

```bash
pip install dcp-client


# With SSE support


pip install dcp-client[sse]
```

## Quick Start

```python
import asyncio from dcp_client import DcpClient async def main():


# Connect via TCP


async with await DcpClient.connect_tcp("localhost", 9000) as client:


# Initialize connection


await client.initialize()


# List available tools


tools = await client.list_tools()
print(f"Available tools: {tools}")


# Call a tool


result = await client.call_tool("my_tool", {"arg": "value"})
print(f"Result: {result}")
asyncio.run(main())
```

## Connection Methods

### TCP Connection

```python
client = await DcpClient.connect_tcp("localhost", 9000)
```

### Stdio Connection (subprocess)

```python
client = await DcpClient.connect_stdio(["./my-server", "--stdio"])
```

### SSE Connection (web)

```python
client = await DcpClient.connect_sse("http://localhost:8080")
```

## API Reference

### Tools

- `list_tools()`
- List available tools
- `call_tool(name, arguments)`
- Call a tool

### Resources

- `list_resources(cursor=None)`
- List resources with pagination
- `read_resource(uri)`
- Read a resource
- `subscribe_resource(uri)`
- Subscribe to changes
- `unsubscribe_resource(uri)`
- Unsubscribe from changes

### Prompts

- `list_prompts()`
- List available prompts
- `get_prompt(name, arguments)`
- Get a rendered prompt

### Logging

- `set_log_level(level)`
- Set server log level

### Sampling

- `create_message(messages,...)`
- Create LLM message

### Completion

- `complete(ref, argument)`
- Get completions

## License

MIT
