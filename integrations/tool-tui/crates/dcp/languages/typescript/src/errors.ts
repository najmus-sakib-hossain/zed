/**
 * DCP Client error types.
 */

export class DcpError extends Error {
    public readonly code?: number;
    public readonly data?: unknown;

    constructor(message: string, code?: number, data?: unknown) {
        super(message);
        this.name = "DcpError";
        this.code = code;
        this.data = data;
    }
}

export class ConnectionError extends DcpError {
    constructor(message: string) {
        super(message);
        this.name = "ConnectionError";
    }
}

export class TimeoutError extends DcpError {
    constructor(message: string) {
        super(message);
        this.name = "TimeoutError";
    }
}

export class ProtocolError extends DcpError {
    constructor(message: string, code?: number) {
        super(message, code);
        this.name = "ProtocolError";
    }
}

export class MethodNotFoundError extends DcpError {
    public readonly method: string;

    constructor(method: string) {
        super(`Method not found: ${method}`, -32601);
        this.name = "MethodNotFoundError";
        this.method = method;
    }
}

export class InvalidParamsError extends DcpError {
    constructor(message: string = "Invalid params") {
        super(message, -32602);
        this.name = "InvalidParamsError";
    }
}
