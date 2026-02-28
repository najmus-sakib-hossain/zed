"""DCP Client SDK for Python.

A native Python SDK for the Development Context Protocol (DCP),
providing async/await support for all MCP operations.
"""

from .client import (
    DcpClient,
    ProtocolVersion,
    Root,
    ElicitationRequest,
    ElicitationResponse,
    ResourceTemplate,
)
from .transport import Transport, TcpTransport, StdioTransport, SseTransport
from .errors import DcpError, ConnectionError, TimeoutError, ProtocolError

__version__ = "0.1.0"
__all__ = [
    "DcpClient",
    "ProtocolVersion",
    "Root",
    "ElicitationRequest",
    "ElicitationResponse",
    "ResourceTemplate",
    "Transport",
    "TcpTransport",
    "StdioTransport",
    "SseTransport",
    "DcpError",
    "ConnectionError",
    "TimeoutError",
    "ProtocolError",
]
