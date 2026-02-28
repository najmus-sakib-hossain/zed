/**
 * Machine Format Serializer for DX Serializer VS Code Extension
 * 
 * Converts DxDocument to compact binary format for machine consumption.
 * This format is optimized for:
 * - Fast parsing by compilers and programming languages
 * - Minimal file size
 * - Direct memory mapping
 * 
 * Requirements: 4.3
 */

import { DxDocument, DxValue, DxSection, createDocument, createSection, strValue, numValue, boolValue, nullValue, arrValue, toFieldDefs, getFieldName } from './llmParser';

// ============================================================================
// Machine Value Types (JSON-based for compatibility)
// ============================================================================

export interface MachineValue {
    t: 's' | 'n' | 'b' | 'x' | 'a' | 'r';
    v: string | number | boolean | null | MachineValue[];
}

export interface MachineSection {
    id: string;
    schema: string[];
    rows: MachineValue[][];
}

export interface MachineDocument {
    version: number;
    context: Record<string, MachineValue>;
    refs: Record<string, string>;
    sections: Record<string, MachineSection>;
}

// ============================================================================
// Value Conversion
// ============================================================================

export function dxValueToMachine(value: DxValue): MachineValue {
    switch (value.type) {
        case 'string':
            return { t: 's', v: String(value.value) };
        case 'number':
            return { t: 'n', v: value.value as number };
        case 'bool':
            return { t: 'b', v: value.value as boolean };
        case 'null':
            return { t: 'x', v: null };
        case 'array': {
            const items = value.value as DxValue[];
            return { t: 'a', v: items.map(dxValueToMachine) };
        }
        case 'ref':
            return { t: 'r', v: value.refKey || String(value.value) };
        default:
            return { t: 's', v: String(value.value) };
    }
}

export function machineValueToDx(value: MachineValue): DxValue {
    switch (value.t) {
        case 's':
            return strValue(String(value.v));
        case 'n':
            return numValue(value.v as number);
        case 'b':
            return boolValue(value.v as boolean);
        case 'x':
            return nullValue();
        case 'a': {
            const items = (value.v as MachineValue[]).map(machineValueToDx);
            return arrValue(items);
        }
        case 'r':
            return { type: 'ref', value: String(value.v), refKey: String(value.v) };
        default:
            return strValue(String(value.v));
    }
}

// ============================================================================
// Document Conversion
// ============================================================================

export function documentToMachine(doc: DxDocument): MachineDocument {
    const context: Record<string, MachineValue> = {};
    for (const [key, value] of doc.context) {
        context[key] = dxValueToMachine(value);
    }

    const refs: Record<string, string> = {};
    for (const [key, value] of doc.refs) {
        refs[key] = value;
    }

    const sections: Record<string, MachineSection> = {};
    for (const [id, section] of doc.sections) {
        sections[id] = {
            id: section.id,
            schema: section.schema.map(f => getFieldName(f)),
            rows: section.rows.map(row => row.map(dxValueToMachine)),
        };
    }

    return { version: 1, context, refs, sections };
}

export function machineToDocument(machine: MachineDocument): DxDocument {
    const doc = createDocument();

    for (const [key, value] of Object.entries(machine.context)) {
        doc.context.set(key, machineValueToDx(value));
    }

    for (const [key, value] of Object.entries(machine.refs)) {
        doc.refs.set(key, value);
    }

    for (const [id, section] of Object.entries(machine.sections)) {
        const dxSection = createSection(id, toFieldDefs(section.schema));
        for (const row of section.rows) {
            dxSection.rows.push(row.map(machineValueToDx));
        }
        doc.sections.set(id, dxSection);
    }

    return doc;
}


// ============================================================================
// JSON Serialization
// ============================================================================

export function serializeMachine(doc: DxDocument): string {
    const machine = documentToMachine(doc);
    return JSON.stringify(machine);
}

export interface DeserializeResult {
    success: boolean;
    document?: DxDocument;
    error?: string;
}

export function deserializeMachine(json: string): DeserializeResult {
    try {
        const machine = JSON.parse(json) as MachineDocument;
        const doc = machineToDocument(machine);
        return { success: true, document: doc };
    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error),
        };
    }
}

// ============================================================================
// Binary Format
// ============================================================================

const BINARY_MAGIC = new Uint8Array([0x44, 0x58, 0x4D, 0x01]);
const BINARY_VERSION = 1;

const enum BinaryValueType {
    Null = 0x00, Bool = 0x01, Int8 = 0x02, Int16 = 0x03,
    Int32 = 0x04, Float64 = 0x06, String = 0x07, Array = 0x08, Ref = 0x09,
}

const enum BinarySectionType {
    Context = 0x01, References = 0x02, Data = 0x03,
}

class BinaryWriter {
    private buffer: number[] = [];

    writeBytes(bytes: Uint8Array): void {
        for (const byte of bytes) this.buffer.push(byte);
    }
    writeByte(value: number): void {
        this.buffer.push(value & 0xFF);
    }
    writeU16(value: number): void {
        this.buffer.push(value & 0xFF);
        this.buffer.push((value >> 8) & 0xFF);
    }
    writeU32(value: number): void {
        this.buffer.push(value & 0xFF);
        this.buffer.push((value >> 8) & 0xFF);
        this.buffer.push((value >> 16) & 0xFF);
        this.buffer.push((value >> 24) & 0xFF);
    }
    writeF64(value: number): void {
        const view = new DataView(new ArrayBuffer(8));
        view.setFloat64(0, value, true);
        for (let i = 0; i < 8; i++) this.buffer.push(view.getUint8(i));
    }
    writeString(str: string): void {
        const bytes = new TextEncoder().encode(str);
        this.writeU16(bytes.length);
        this.writeBytes(bytes);
    }
    writeShortString(str: string): void {
        const bytes = new TextEncoder().encode(str);
        this.writeByte(Math.min(bytes.length, 255));
        this.writeBytes(bytes.slice(0, 255));
    }
    toBuffer(): Uint8Array {
        return new Uint8Array(this.buffer);
    }
}

function writeBinaryValue(writer: BinaryWriter, value: DxValue): void {
    switch (value.type) {
        case 'null':
            writer.writeByte(BinaryValueType.Null);
            break;
        case 'bool':
            writer.writeByte(BinaryValueType.Bool);
            writer.writeByte(value.value ? 1 : 0);
            break;
        case 'number': {
            const num = value.value as number;
            if (Number.isInteger(num) && num >= -128 && num <= 127) {
                writer.writeByte(BinaryValueType.Int8);
                writer.writeByte(num & 0xFF);
            } else if (Number.isInteger(num) && num >= -32768 && num <= 32767) {
                writer.writeByte(BinaryValueType.Int16);
                writer.writeU16(num & 0xFFFF);
            } else if (Number.isInteger(num)) {
                writer.writeByte(BinaryValueType.Int32);
                writer.writeU32(num >>> 0);
            } else {
                writer.writeByte(BinaryValueType.Float64);
                writer.writeF64(num);
            }
            break;
        }
        case 'string':
            writer.writeByte(BinaryValueType.String);
            writer.writeString(String(value.value));
            break;
        case 'array': {
            writer.writeByte(BinaryValueType.Array);
            const items = value.value as DxValue[];
            writer.writeU16(items.length);
            for (const item of items) writeBinaryValue(writer, item);
            break;
        }
        case 'ref':
            writer.writeByte(BinaryValueType.Ref);
            writer.writeShortString(value.refKey || String(value.value));
            break;
        default:
            writer.writeByte(BinaryValueType.String);
            writer.writeString(String(value.value));
    }
}

export function serializeToBinary(doc: DxDocument): Uint8Array {
    const writer = new BinaryWriter();
    writer.writeBytes(BINARY_MAGIC);
    writer.writeByte(BINARY_VERSION);
    writer.writeByte(0);

    let sectionCount = 0;
    if (doc.context.size > 0) sectionCount++;
    if (doc.refs.size > 0) sectionCount++;
    sectionCount += doc.sections.size;
    writer.writeU16(sectionCount);

    if (doc.context.size > 0) {
        writer.writeByte(BinarySectionType.Context);
        writer.writeU16(doc.context.size);
        for (const [key, value] of doc.context) {
            writer.writeShortString(key);
            writeBinaryValue(writer, value);
        }
    }

    if (doc.refs.size > 0) {
        writer.writeByte(BinarySectionType.References);
        writer.writeU16(doc.refs.size);
        for (const [key, value] of doc.refs) {
            writer.writeShortString(key);
            writer.writeString(value);
        }
    }

    for (const [id, section] of doc.sections) {
        writer.writeByte(BinarySectionType.Data);
        writer.writeShortString(id);
        writer.writeU16(section.schema.length);
        for (const col of section.schema) writer.writeShortString(getFieldName(col));
        writer.writeU16(section.rows.length);
        for (const row of section.rows) {
            for (const cell of row) writeBinaryValue(writer, cell);
        }
    }

    return writer.toBuffer();
}

export function verifyBinary(buffer: Uint8Array): boolean {
    if (buffer.length < 8) return false;
    for (let i = 0; i < 4; i++) {
        if (buffer[i] !== BINARY_MAGIC[i]) return false;
    }
    return buffer[4] === BINARY_VERSION;
}

export function getBinaryInfo(buffer: Uint8Array): { valid: boolean; version?: number; sectionCount?: number; size: number } {
    if (!verifyBinary(buffer)) return { valid: false, size: buffer.length };
    return {
        valid: true,
        version: buffer[4],
        sectionCount: buffer[6] | (buffer[7] << 8),
        size: buffer.length,
    };
}
