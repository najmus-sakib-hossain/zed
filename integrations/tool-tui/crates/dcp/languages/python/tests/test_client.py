"""Tests for DcpClient."""

import asyncio
import json
import pytest
from unittest.mock import AsyncMock, MagicMock

from dcp_client import DcpClient
from dcp_client.transport import Transport
from dcp_client.errors import DcpError, TimeoutError


class MockTransport(Transport):
    """Mock transport for testing."""
    
    def __init__(self):
        self._connected = False
        self._messages: list = []
        self._responses: list = []
        self._receive_queue: asyncio.Queue = asyncio.Queue()
    
    async def connect(self) -> None:
        self._connected = True
    
    async def send(self, message: str) -> None:
        self._messages.append(json.loads(message))
        # Auto-respond if we have queued responses
        if self._responses:
            response = self._responses.pop(0)
            await self._receive_queue.put(json.dumps(response))
    
    async def receive(self) -> str:
        return await self._receive_queue.get()
    
    async def close(self) -> None:
        self._connected = False
    
    @property
    def is_connected(self) -> bool:
        return self._connected
    
    def queue_response(self, response: dict) -> None:
        """Queue a response to be returned."""
        self._responses.append(response)


@pytest.fixture
def mock_transport():
    return MockTransport()


@pytest.fixture
async def client(mock_transport):
    await mock_transport.connect()
    c = DcpClient(mock_transport, timeout=1.0)
    c._start_receive_loop()
    yield c
    await c.close()


@pytest.mark.asyncio
async def test_initialize(client, mock_transport):
    """Test initialize method."""
    mock_transport.queue_response({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "test-server", "version": "1.0.0"}
        }
    })
    
    result = await client.initialize()
    
    assert result["protocolVersion"] == "2024-11-05"
    assert "capabilities" in result
    assert client._initialized


@pytest.mark.asyncio
async def test_list_tools(client, mock_transport):
    """Test list_tools method."""
    mock_transport.queue_response({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "tools": [
                {"name": "tool1", "description": "First tool"},
                {"name": "tool2", "description": "Second tool"}
            ]
        }
    })
    
    tools = await client.list_tools()
    
    assert len(tools) == 2
    assert tools[0]["name"] == "tool1"
    assert tools[1]["name"] == "tool2"


@pytest.mark.asyncio
async def test_call_tool(client, mock_transport):
    """Test call_tool method."""
    mock_transport.queue_response({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [{"type": "text", "text": "Hello, World!"}]
        }
    })
    
    result = await client.call_tool("greet", {"name": "World"})
    
    assert "content" in result
    assert result["content"][0]["text"] == "Hello, World!"
    
    # Verify request was sent correctly
    request = mock_transport._messages[0]
    assert request["method"] == "tools/call"
    assert request["params"]["name"] == "greet"
    assert request["params"]["arguments"]["name"] == "World"


@pytest.mark.asyncio
async def test_list_resources(client, mock_transport):
    """Test list_resources method."""
    mock_transport.queue_response({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "resources": [
                {"uri": "file:///test.txt", "name": "test.txt"}
            ],
            "nextCursor": "cursor123"
        }
    })
    
    result = await client.list_resources()
    
    assert len(result["resources"]) == 1
    assert result["resources"][0]["uri"] == "file:///test.txt"
    assert result["nextCursor"] == "cursor123"


@pytest.mark.asyncio
async def test_read_resource(client, mock_transport):
    """Test read_resource method."""
    mock_transport.queue_response({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "contents": [{"uri": "file:///test.txt", "text": "Hello"}]
        }
    })
    
    result = await client.read_resource("file:///test.txt")
    
    assert "contents" in result
    
    request = mock_transport._messages[0]
    assert request["params"]["uri"] == "file:///test.txt"


@pytest.mark.asyncio
async def test_list_prompts(client, mock_transport):
    """Test list_prompts method."""
    mock_transport.queue_response({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "prompts": [
                {"name": "greeting", "description": "A greeting prompt"}
            ]
        }
    })
    
    prompts = await client.list_prompts()
    
    assert len(prompts) == 1
    assert prompts[0]["name"] == "greeting"


@pytest.mark.asyncio
async def test_get_prompt(client, mock_transport):
    """Test get_prompt method."""
    mock_transport.queue_response({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "messages": [{"role": "user", "content": {"type": "text", "text": "Hello, Alice!"}}]
        }
    })
    
    result = await client.get_prompt("greeting", {"name": "Alice"})
    
    assert "messages" in result
    
    request = mock_transport._messages[0]
    assert request["params"]["name"] == "greeting"
    assert request["params"]["arguments"]["name"] == "Alice"


@pytest.mark.asyncio
async def test_error_response(client, mock_transport):
    """Test error handling."""
    mock_transport.queue_response({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32601,
            "message": "Method not found"
        }
    })
    
    with pytest.raises(DcpError) as exc_info:
        await client._request("unknown/method")
    
    assert exc_info.value.code == -32601
    assert "Method not found" in str(exc_info.value)


@pytest.mark.asyncio
async def test_request_id_increments(client, mock_transport):
    """Test that request IDs increment."""
    mock_transport.queue_response({"jsonrpc": "2.0", "id": 1, "result": {}})
    mock_transport.queue_response({"jsonrpc": "2.0", "id": 2, "result": {}})
    mock_transport.queue_response({"jsonrpc": "2.0", "id": 3, "result": {}})
    
    await client._request("test1")
    await client._request("test2")
    await client._request("test3")
    
    assert mock_transport._messages[0]["id"] == 1
    assert mock_transport._messages[1]["id"] == 2
    assert mock_transport._messages[2]["id"] == 3
