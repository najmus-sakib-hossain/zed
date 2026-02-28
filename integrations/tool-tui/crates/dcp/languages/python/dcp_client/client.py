"""DCP Client implementation with async/await support."""

import asyncio
import json
from typing import Any, Callable, Dict, List, Optional, Union
from dataclasses import dataclass
from enum import Enum

from .transport import Transport, TcpTransport, StdioTransport, SseTransport
from .errors import DcpError, ConnectionError, TimeoutError, ProtocolError


class ProtocolVersion(Enum):
    """Supported MCP protocol versions."""
    V2024_11_05 = "2024-11-05"
    V2025_03_26 = "2025-03-26"
    V2025_06_18 = "2025-06-18"
    
    @classmethod
    def from_str(cls, version: str) -> Optional["ProtocolVersion"]:
        """Parse version string to enum."""
        for v in cls:
            if v.value == version:
                return v
        return None
    
    def supports_roots(self) -> bool:
        """Check if version supports roots capability."""
        return self in (ProtocolVersion.V2025_03_26, ProtocolVersion.V2025_06_18)
    
    def supports_elicitation(self) -> bool:
        """Check if version supports elicitation capability."""
        return self == ProtocolVersion.V2025_06_18


@dataclass
class Root:
    """Root definition for filesystem boundaries."""
    uri: str
    name: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        result = {"uri": self.uri}
        if self.name is not None:
            result["name"] = self.name
        return result
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Root":
        """Create Root from dictionary."""
        return cls(uri=data["uri"], name=data.get("name"))


@dataclass
class ElicitationRequest:
    """Elicitation request for server-initiated user input."""
    message: str
    requested_schema: Optional[Dict[str, Any]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        result = {"message": self.message}
        if self.requested_schema is not None:
            result["requestedSchema"] = self.requested_schema
        return result


@dataclass
class ElicitationResponse:
    """Elicitation response with action and optional content."""
    action: str  # "accept", "decline", or "cancel"
    content: Optional[Dict[str, Any]] = None
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "ElicitationResponse":
        """Create ElicitationResponse from dictionary."""
        return cls(action=data["action"], content=data.get("content"))


@dataclass
class ResourceTemplate:
    """Resource template with URI pattern."""
    uri_template: str
    name: str
    description: Optional[str] = None
    mime_type: Optional[str] = None
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "ResourceTemplate":
        """Create ResourceTemplate from dictionary."""
        return cls(
            uri_template=data["uriTemplate"],
            name=data["name"],
            description=data.get("description"),
            mime_type=data.get("mimeType")
        )
    
    def substitute(self, params: Dict[str, str]) -> str:
        """Substitute template parameters to get concrete URI."""
        result = self.uri_template
        for key, value in params.items():
            result = result.replace(f"{{{key}}}", value)
        return result


class DcpClient:
    """DCP client with async/await support for all MCP operations."""
    
    # Default to latest protocol version
    DEFAULT_PROTOCOL_VERSION = ProtocolVersion.V2025_06_18
    
    def __init__(self, transport: Transport, timeout: float = 30.0, 
                 protocol_version: Optional[ProtocolVersion] = None):
        """Initialize DCP client with a transport.
        
        Args:
            transport: The transport to use for communication
            timeout: Default timeout for requests in seconds
            protocol_version: Preferred protocol version (defaults to latest)
        """
        self.transport = transport
        self.timeout = timeout
        self._preferred_version = protocol_version or self.DEFAULT_PROTOCOL_VERSION
        self._negotiated_version: Optional[ProtocolVersion] = None
        self._request_id = 0
        self._pending: Dict[int, asyncio.Future] = {}
        self._notification_handlers: Dict[str, Callable] = {}
        self._receive_task: Optional[asyncio.Task] = None
        self._initialized = False
        self._server_capabilities: Dict[str, Any] = {}
        self._roots_changed_callback: Optional[Callable[[List[Root]], Any]] = None
    
    @classmethod
    async def connect_tcp(cls, host: str, port: int, timeout: float = 30.0,
                          protocol_version: Optional[ProtocolVersion] = None) -> "DcpClient":
        """Connect via TCP.
        
        Args:
            host: Server hostname
            port: Server port
            timeout: Connection timeout
            protocol_version: Preferred protocol version
            
        Returns:
            Connected DcpClient instance
        """
        transport = await TcpTransport.connect_to(host, port, timeout)
        client = cls(transport, timeout, protocol_version)
        client._start_receive_loop()
        return client

    @classmethod
    async def connect_stdio(cls, command: List[str], timeout: float = 30.0,
                            protocol_version: Optional[ProtocolVersion] = None) -> "DcpClient":
        """Connect via stdio to a subprocess.
        
        Args:
            command: Command and arguments to spawn
            timeout: Request timeout
            protocol_version: Preferred protocol version
            
        Returns:
            Connected DcpClient instance
        """
        transport = await StdioTransport.spawn(command)
        client = cls(transport, timeout, protocol_version)
        client._start_receive_loop()
        return client
    
    @classmethod
    async def connect_sse(cls, url: str, timeout: float = 30.0,
                          protocol_version: Optional[ProtocolVersion] = None) -> "DcpClient":
        """Connect via Server-Sent Events.
        
        Args:
            url: SSE endpoint URL
            timeout: Request timeout
            protocol_version: Preferred protocol version
            
        Returns:
            Connected DcpClient instance
        """
        transport = await SseTransport.connect_to(url, timeout)
        client = cls(transport, timeout, protocol_version)
        client._start_receive_loop()
        return client
    
    def _start_receive_loop(self) -> None:
        """Start the background receive loop."""
        self._receive_task = asyncio.create_task(self._receive_loop())
    
    async def _receive_loop(self) -> None:
        """Background loop to receive and dispatch messages."""
        while self.transport.is_connected:
            try:
                message = await self.transport.receive()
                if message is None:
                    break
                await self._handle_message(message)
            except Exception:
                break
    
    async def _handle_message(self, message: str) -> None:
        """Handle an incoming message."""
        try:
            data = json.loads(message)
        except json.JSONDecodeError:
            return
        
        # Check if it's a response (has id)
        if "id" in data and data["id"] is not None:
            request_id = data["id"]
            if request_id in self._pending:
                future = self._pending.pop(request_id)
                if "error" in data:
                    error = data["error"]
                    future.set_exception(DcpError(
                        error.get("message", "Unknown error"),
                        error.get("code"),
                        error.get("data")
                    ))
                else:
                    future.set_result(data.get("result", {}))
        
        # Check if it's a notification (no id)
        elif "method" in data:
            method = data["method"]
            if method in self._notification_handlers:
                handler = self._notification_handlers[method]
                params = data.get("params", {})
                try:
                    await handler(params)
                except Exception:
                    pass

    async def _request(self, method: str, params: Optional[Dict[str, Any]] = None) -> Any:
        """Send a JSON-RPC request and wait for response.
        
        Args:
            method: The method name
            params: Optional parameters
            
        Returns:
            The result from the response
        """
        self._request_id += 1
        request_id = self._request_id
        
        request = {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
        }
        if params is not None:
            request["params"] = params
        
        future: asyncio.Future = asyncio.get_event_loop().create_future()
        self._pending[request_id] = future
        
        try:
            await self.transport.send(json.dumps(request))
            return await asyncio.wait_for(future, timeout=self.timeout)
        except asyncio.TimeoutError:
            self._pending.pop(request_id, None)
            raise TimeoutError(f"Request {method} timed out")
    
    async def _notify(self, method: str, params: Optional[Dict[str, Any]] = None) -> None:
        """Send a JSON-RPC notification (no response expected).
        
        Args:
            method: The method name
            params: Optional parameters
        """
        notification = {
            "jsonrpc": "2.0",
            "method": method,
        }
        if params is not None:
            notification["params"] = params
        
        await self.transport.send(json.dumps(notification))
    
    def on_notification(self, method: str, handler: Callable) -> None:
        """Register a handler for notifications.
        
        Args:
            method: The notification method to handle
            handler: Async function to call with params
        """
        self._notification_handlers[method] = handler
    
    # =========================================================================
    # Lifecycle Methods
    # =========================================================================
    
    async def initialize(self) -> Dict[str, Any]:
        """Initialize connection and negotiate capabilities.
        
        Returns:
            Server capabilities and info
        """
        # Build capabilities based on preferred version
        capabilities: Dict[str, Any] = {}
        if self._preferred_version.supports_roots():
            capabilities["roots"] = {"listChanged": True}
        
        result = await self._request("initialize", {
            "protocolVersion": self._preferred_version.value,
            "capabilities": capabilities,
            "clientInfo": {
                "name": "dcp-python",
                "version": "0.1.0"
            }
        })
        
        # Parse negotiated version
        negotiated_str = result.get("protocolVersion", "2024-11-05")
        self._negotiated_version = ProtocolVersion.from_str(negotiated_str) or ProtocolVersion.V2024_11_05
        
        self._server_capabilities = result.get("capabilities", {})
        self._initialized = True
        
        # Register internal notification handlers for roots changes
        if self._negotiated_version.supports_roots():
            self.on_notification("notifications/roots/list_changed", self._handle_roots_changed)
        
        # Send initialized notification
        await self._notify("notifications/initialized")
        
        return result
    
    async def _handle_roots_changed(self, params: Dict[str, Any]) -> None:
        """Handle roots list changed notification."""
        if self._roots_changed_callback:
            roots = await self.list_roots()
            await self._roots_changed_callback(roots)
    
    @property
    def negotiated_version(self) -> Optional[ProtocolVersion]:
        """Get the negotiated protocol version."""
        return self._negotiated_version
    
    @property
    def server_capabilities(self) -> Dict[str, Any]:
        """Get server capabilities from initialization."""
        return self._server_capabilities

    # =========================================================================
    # Tool Methods
    # =========================================================================
    
    async def list_tools(self) -> List[Dict[str, Any]]:
        """List available tools.
        
        Returns:
            List of tool definitions
        """
        result = await self._request("tools/list", {})
        return result.get("tools", [])
    
    async def call_tool(self, name: str, arguments: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """Call a tool by name.
        
        Args:
            name: Tool name
            arguments: Tool arguments
            
        Returns:
            Tool execution result
        """
        params = {"name": name}
        if arguments is not None:
            params["arguments"] = arguments
        return await self._request("tools/call", params)
    
    # =========================================================================
    # Resource Methods
    # =========================================================================
    
    async def list_resources(self, cursor: Optional[str] = None) -> Dict[str, Any]:
        """List available resources.
        
        Args:
            cursor: Pagination cursor
            
        Returns:
            Resources list with optional next_cursor
        """
        params = {}
        if cursor is not None:
            params["cursor"] = cursor
        return await self._request("resources/list", params)
    
    async def read_resource(self, uri: str) -> Dict[str, Any]:
        """Read a resource by URI.
        
        Args:
            uri: Resource URI
            
        Returns:
            Resource content
        """
        return await self._request("resources/read", {"uri": uri})
    
    async def subscribe_resource(self, uri: str) -> None:
        """Subscribe to resource changes.
        
        Args:
            uri: Resource URI to subscribe to
        """
        await self._request("resources/subscribe", {"uri": uri})
    
    async def unsubscribe_resource(self, uri: str) -> None:
        """Unsubscribe from resource changes.
        
        Args:
            uri: Resource URI to unsubscribe from
        """
        await self._request("resources/unsubscribe", {"uri": uri})
    
    # =========================================================================
    # Prompt Methods
    # =========================================================================
    
    async def list_prompts(self) -> List[Dict[str, Any]]:
        """List available prompts.
        
        Returns:
            List of prompt definitions
        """
        result = await self._request("prompts/list", {})
        return result.get("prompts", [])
    
    async def get_prompt(self, name: str, arguments: Optional[Dict[str, str]] = None) -> Dict[str, Any]:
        """Get a prompt with arguments.
        
        Args:
            name: Prompt name
            arguments: Prompt arguments
            
        Returns:
            Rendered prompt
        """
        params = {"name": name}
        if arguments is not None:
            params["arguments"] = arguments
        return await self._request("prompts/get", params)

    # =========================================================================
    # Roots Methods (MCP 2025-03-26+)
    # =========================================================================
    
    async def list_roots(self) -> List[Root]:
        """List configured roots (filesystem boundaries).
        
        Requires protocol version 2025-03-26 or later.
        
        Returns:
            List of Root objects
            
        Raises:
            DcpError: If roots capability is not supported
        """
        if self._negotiated_version and not self._negotiated_version.supports_roots():
            raise DcpError(
                "Roots not supported in negotiated protocol version",
                code=-32601,
                data={"requiredVersion": "2025-03-26", 
                      "negotiatedVersion": self._negotiated_version.value}
            )
        
        result = await self._request("roots/list", {})
        roots_data = result.get("roots", [])
        return [Root.from_dict(r) for r in roots_data]
    
    def on_roots_changed(self, callback: Callable[[List[Root]], Any]) -> None:
        """Register a callback for roots list changes.
        
        The callback will be invoked when the server emits a
        notifications/roots/list_changed notification.
        
        Args:
            callback: Async function to call with updated roots list
        """
        self._roots_changed_callback = callback
    
    # =========================================================================
    # Elicitation Methods (MCP 2025-06-18+)
    # =========================================================================
    
    async def handle_elicitation(
        self,
        handler: Callable[[ElicitationRequest], ElicitationResponse]
    ) -> None:
        """Register a handler for elicitation requests from the server.
        
        Requires protocol version 2025-06-18 or later.
        
        Args:
            handler: Function that receives ElicitationRequest and returns ElicitationResponse
        """
        if self._negotiated_version and not self._negotiated_version.supports_elicitation():
            raise DcpError(
                "Elicitation not supported in negotiated protocol version",
                code=-32601,
                data={"requiredVersion": "2025-06-18",
                      "negotiatedVersion": self._negotiated_version.value if self._negotiated_version else None}
            )
        
        async def elicitation_notification_handler(params: Dict[str, Any]) -> None:
            request = ElicitationRequest(
                message=params.get("message", ""),
                requested_schema=params.get("requestedSchema")
            )
            response = handler(request)
            # Send response back via notification
            await self._notify("elicitation/respond", {
                "action": response.action,
                "content": response.content
            })
        
        self.on_notification("elicitation/create", elicitation_notification_handler)
    
    # =========================================================================
    # Resource Template Methods (MCP 2025-03-26+)
    # =========================================================================
    
    async def list_resource_templates(self) -> List[ResourceTemplate]:
        """List available resource templates.
        
        Resource templates are included in the resources/list response
        for protocol versions 2025-03-26 and later.
        
        Returns:
            List of ResourceTemplate objects
        """
        result = await self._request("resources/list", {})
        templates_data = result.get("resourceTemplates", [])
        return [ResourceTemplate.from_dict(t) for t in templates_data]
    
    async def read_resource_template(
        self, 
        template: ResourceTemplate, 
        params: Dict[str, str]
    ) -> Dict[str, Any]:
        """Read a resource by substituting template parameters.
        
        Args:
            template: The resource template to use
            params: Parameters to substitute into the template
            
        Returns:
            Resource content
        """
        uri = template.substitute(params)
        return await self.read_resource(uri)

    # =========================================================================
    # Logging Methods
    # =========================================================================
    
    async def set_log_level(self, level: str) -> None:
        """Set the server log level.
        
        Args:
            level: Log level (debug, info, warn, error)
        """
        await self._request("logging/setLevel", {"level": level})
    
    # =========================================================================
    # Sampling Methods
    # =========================================================================
    
    async def create_message(
        self,
        messages: List[Dict[str, Any]],
        model_preferences: Optional[Dict[str, Any]] = None,
        system_prompt: Optional[str] = None,
        max_tokens: int = 1024
    ) -> Dict[str, Any]:
        """Create a message using LLM sampling.
        
        Args:
            messages: Conversation messages
            model_preferences: Model selection preferences
            system_prompt: Optional system prompt
            max_tokens: Maximum tokens to generate
            
        Returns:
            Generated message
        """
        params = {
            "messages": messages,
            "maxTokens": max_tokens
        }
        if model_preferences is not None:
            params["modelPreferences"] = model_preferences
        if system_prompt is not None:
            params["systemPrompt"] = system_prompt
        
        return await self._request("sampling/createMessage", params)
    
    # =========================================================================
    # Completion Methods
    # =========================================================================
    
    async def complete(
        self,
        ref: Dict[str, str],
        argument: Dict[str, str]
    ) -> Dict[str, Any]:
        """Get completions for an argument.
        
        Args:
            ref: Reference (type and name)
            argument: Argument to complete
            
        Returns:
            Completion suggestions
        """
        return await self._request("completion/complete", {
            "ref": ref,
            "argument": argument
        })
    
    # =========================================================================
    # Connection Management
    # =========================================================================
    
    async def close(self) -> None:
        """Close the connection."""
        if self._receive_task:
            self._receive_task.cancel()
            try:
                await self._receive_task
            except asyncio.CancelledError:
                pass
        
        await self.transport.close()
        self._initialized = False
    
    async def reconnect(self) -> None:
        """Reconnect to the server."""
        await self.transport.close()
        await self.transport.connect()
        self._start_receive_loop()
        
        # Re-initialize if we were initialized before
        if self._initialized:
            await self.initialize()
    
    @property
    def is_connected(self) -> bool:
        """Check if client is connected."""
        return self.transport.is_connected
    
    async def __aenter__(self) -> "DcpClient":
        """Async context manager entry."""
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        """Async context manager exit."""
        await self.close()
