"""Transport implementations for DCP client."""

import asyncio
import json
import subprocess
from abc import ABC, abstractmethod
from typing import Any, Callable, List, Optional

from .errors import ConnectionError, ProtocolError


class Transport(ABC):
    """Abstract base class for DCP transports."""
    
    @abstractmethod
    async def connect(self) -> None:
        """Establish connection."""
        pass
    
    @abstractmethod
    async def send(self, message: str) -> None:
        """Send a message."""
        pass
    
    @abstractmethod
    async def receive(self) -> Optional[str]:
        """Receive a message. Returns None on EOF."""
        pass
    
    @abstractmethod
    async def close(self) -> None:
        """Close the connection."""
        pass
    
    @property
    @abstractmethod
    def is_connected(self) -> bool:
        """Check if transport is connected."""
        pass


class TcpTransport(Transport):
    """TCP transport for DCP connections."""
    
    def __init__(self, host: str, port: int, timeout: float = 30.0):
        self.host = host
        self.port = port
        self.timeout = timeout
        self._reader: Optional[asyncio.StreamReader] = None
        self._writer: Optional[asyncio.StreamWriter] = None
        self._connected = False

    @classmethod
    async def connect_to(cls, host: str, port: int, timeout: float = 30.0) -> "TcpTransport":
        """Create and connect a TCP transport."""
        transport = cls(host, port, timeout)
        await transport.connect()
        return transport
    
    async def connect(self) -> None:
        """Establish TCP connection."""
        try:
            self._reader, self._writer = await asyncio.wait_for(
                asyncio.open_connection(self.host, self.port),
                timeout=self.timeout
            )
            self._connected = True
        except asyncio.TimeoutError:
            raise ConnectionError(f"Connection to {self.host}:{self.port} timed out")
        except OSError as e:
            raise ConnectionError(f"Failed to connect to {self.host}:{self.port}: {e}")
    
    async def send(self, message: str) -> None:
        """Send a message over TCP (newline-delimited)."""
        if not self._connected or self._writer is None:
            raise ConnectionError("Not connected")
        
        data = (message + "\n").encode("utf-8")
        self._writer.write(data)
        await self._writer.drain()
    
    async def receive(self) -> Optional[str]:
        """Receive a message from TCP."""
        if not self._connected or self._reader is None:
            raise ConnectionError("Not connected")
        
        try:
            line = await self._reader.readline()
            if not line:
                return None
            return line.decode("utf-8").strip()
        except Exception as e:
            raise ConnectionError(f"Failed to receive: {e}")
    
    async def close(self) -> None:
        """Close TCP connection."""
        if self._writer:
            self._writer.close()
            await self._writer.wait_closed()
        self._connected = False
        self._reader = None
        self._writer = None
    
    @property
    def is_connected(self) -> bool:
        return self._connected


class StdioTransport(Transport):
    """Stdio transport for subprocess communication."""
    
    def __init__(self, command: List[str]):
        self.command = command
        self._process: Optional[subprocess.Popen] = None
        self._connected = False
    
    @classmethod
    async def spawn(cls, command: List[str]) -> "StdioTransport":
        """Spawn a subprocess and create stdio transport."""
        transport = cls(command)
        await transport.connect()
        return transport
    
    async def connect(self) -> None:
        """Start the subprocess."""
        try:
            self._process = await asyncio.create_subprocess_exec(
                *self.command,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            self._connected = True
        except Exception as e:
            raise ConnectionError(f"Failed to spawn process: {e}")
    
    async def send(self, message: str) -> None:
        """Send a message to subprocess stdin."""
        if not self._connected or self._process is None or self._process.stdin is None:
            raise ConnectionError("Not connected")
        
        data = (message + "\n").encode("utf-8")
        self._process.stdin.write(data)
        await self._process.stdin.drain()
    
    async def receive(self) -> Optional[str]:
        """Receive a message from subprocess stdout."""
        if not self._connected or self._process is None or self._process.stdout is None:
            raise ConnectionError("Not connected")
        
        line = await self._process.stdout.readline()
        if not line:
            return None
        return line.decode("utf-8").strip()
    
    async def close(self) -> None:
        """Terminate the subprocess."""
        if self._process:
            self._process.terminate()
            await self._process.wait()
        self._connected = False
        self._process = None
    
    @property
    def is_connected(self) -> bool:
        return self._connected and self._process is not None


class SseTransport(Transport):
    """Server-Sent Events transport for web compatibility."""
    
    def __init__(self, url: str, timeout: float = 30.0):
        self.url = url
        self.timeout = timeout
        self._connected = False
        self._event_queue: asyncio.Queue[str] = asyncio.Queue()
        self._session = None
        self._sse_task: Optional[asyncio.Task] = None
        self._last_event_id: Optional[str] = None
    
    @classmethod
    async def connect_to(cls, url: str, timeout: float = 30.0) -> "SseTransport":
        """Create and connect an SSE transport."""
        transport = cls(url, timeout)
        await transport.connect()
        return transport
    
    async def connect(self) -> None:
        """Establish SSE connection."""
        try:
            # Import aiohttp only when needed
            import aiohttp
            self._session = aiohttp.ClientSession()
            
            # Start SSE listener task
            self._sse_task = asyncio.create_task(self._listen_sse())
            self._connected = True
        except ImportError:
            raise ConnectionError("aiohttp is required for SSE transport: pip install aiohttp")
        except Exception as e:
            raise ConnectionError(f"Failed to connect to SSE endpoint: {e}")
    
    async def _listen_sse(self) -> None:
        """Listen for SSE events."""
        import aiohttp
        
        headers = {}
        if self._last_event_id:
            headers["Last-Event-ID"] = self._last_event_id
        
        events_url = self.url.rstrip("/") + "/events"
        
        try:
            async with self._session.get(events_url, headers=headers) as response:
                async for line in response.content:
                    line = line.decode("utf-8").strip()
                    if line.startswith("data:"):
                        data = line[5:].strip()
                        await self._event_queue.put(data)
                    elif line.startswith("id:"):
                        self._last_event_id = line[3:].strip()
        except Exception:
            self._connected = False
    
    async def send(self, message: str) -> None:
        """Send a message via HTTP POST."""
        if not self._connected or self._session is None:
            raise ConnectionError("Not connected")
        
        post_url = self.url.rstrip("/") + "/message"
        
        try:
            async with self._session.post(
                post_url,
                data=message,
                headers={"Content-Type": "application/json"}
            ) as response:
                if response.status != 200:
                    raise ProtocolError(f"POST failed with status {response.status}")
        except Exception as e:
            raise ConnectionError(f"Failed to send message: {e}")
    
    async def receive(self) -> Optional[str]:
        """Receive a message from SSE event queue."""
        if not self._connected:
            raise ConnectionError("Not connected")
        
        try:
            return await asyncio.wait_for(self._event_queue.get(), timeout=self.timeout)
        except asyncio.TimeoutError:
            return None
    
    async def close(self) -> None:
        """Close SSE connection."""
        if self._sse_task:
            self._sse_task.cancel()
            try:
                await self._sse_task
            except asyncio.CancelledError:
                pass
        
        if self._session:
            await self._session.close()
        
        self._connected = False
        self._session = None
        self._sse_task = None
    
    @property
    def is_connected(self) -> bool:
        return self._connected
