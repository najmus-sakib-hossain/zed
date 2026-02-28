"""DCP Client error types."""

from typing import Any, Optional


class DcpError(Exception):
    """Base exception for DCP client errors."""
    
    def __init__(self, message: str, code: Optional[int] = None, data: Optional[Any] = None):
        super().__init__(message)
        self.message = message
        self.code = code
        self.data = data


class ConnectionError(DcpError):
    """Connection-related errors."""
    pass


class TimeoutError(DcpError):
    """Request timeout errors."""
    pass


class ProtocolError(DcpError):
    """Protocol-level errors (invalid JSON-RPC, etc.)."""
    pass


class MethodNotFoundError(DcpError):
    """Method not found error (JSON-RPC -32601)."""
    
    def __init__(self, method: str):
        super().__init__(f"Method not found: {method}", code=-32601)
        self.method = method


class InvalidParamsError(DcpError):
    """Invalid parameters error (JSON-RPC -32602)."""
    
    def __init__(self, message: str = "Invalid params"):
        super().__init__(message, code=-32602)
