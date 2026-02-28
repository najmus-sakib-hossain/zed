
# DCP TypeScript SDK

A Promise-based TypeScript client for the Development Context Protocol (DCP).

## Installation

```bash
npm install dcp-client ```


## Quick Start


```typescript
import { DcpClient } from "dcp-client";
// Connect via TCP const client = await DcpClient.connectTcp("localhost", 9000);
// Initialize the connection await client.initialize();
// List available tools const tools = await client.listTools();
console.log("Available tools:", tools);
// Call a tool const result = await client.callTool("my-tool", { arg: "value" });
console.log("Result:", result);
// Close the connection await client.close();
```


## Transport Options



### TCP (Node.js)


```typescript
const client = await DcpClient.connectTcp("localhost", 9000);
```


### Stdio (Node.js)


```typescript
const client = await DcpClient.connectStdio(["./my-server", "--stdio"]);
```


### SSE (Browser)


```typescript
const client = await DcpClient.connectSse("http://localhost:9000");
```


## API Reference



### Lifecycle


- `initialize()`
- Initialize connection and negotiate capabilities
- `close()`
- Close the connection
- `reconnect()`
- Reconnect to the server
- `isConnected`
- Check connection status


### Tools


- `listTools()`
- List available tools
- `callTool(name, args?)`
- Call a tool by name


### Resources


- `listResources(cursor?)`
- List available resources (with pagination)
- `readResource(uri)`
- Read a resource by URI
- `subscribeResource(uri)`
- Subscribe to resource changes
- `unsubscribeResource(uri)`
- Unsubscribe from resource changes


### Prompts


- `listPrompts()`
- List available prompts
- `getPrompt(name, args?)`
- Get a prompt with arguments


### Logging


- `setLogLevel(level)`
- Set the server log level


### Sampling


- `createMessage(params)`
- Create a message using LLM sampling


### Completion


- `complete(params)`
- Get completions for an argument


### Notifications


```typescript
client.onNotification("notifications/resources/updated", (params) => { console.log("Resource updated:", params);
});
```


## Error Handling


```typescript
import { DcpError, TimeoutError, ConnectionError } from "dcp-client";
try { await client.callTool("unknown-tool");
} catch (error) { if (error instanceof DcpError) { console.error("DCP error:", error.code, error.message);
} else if (error instanceof TimeoutError) { console.error("Request timed out");
} else if (error instanceof ConnectionError) { console.error("Connection failed");
}
}
```


## Custom Transport


```typescript
import { DcpClient, Transport } from "dcp-client";
class MyTransport implements Transport { async connect(): Promise<void> { /* ... */ }
async send(message: string): Promise<void> { /* ... */ }
async receive(): Promise<string | null> { /* ... */ }
async close(): Promise<void> { /* ... */ }
get isConnected(): boolean { /* ... */ }
onMessage(handler: (message: string) => void): void { /* ... */ }
}
const client = new DcpClient({ transport: new MyTransport(), timeout: 30000, });
```


## License


MIT
