#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

// Load WASM modules
const markdownWasm = require('./wasm-markdown/markdown.js');
const serializerWasm = require('./wasm-serializer/serializer.js');

async function convertMarkdownFiles() {
    console.log('📝 Converting Markdown .llm files...');
    
    let converted = 0;
    const files = [];
    
    function findFiles(dir) {
        const entries = fs.readdirSync(dir, { withFileTypes: true });
        for (const entry of entries) {
            const fullPath = path.join(dir, entry.name);
            if (entry.isDirectory()) {
                findFiles(fullPath);
            } else if (entry.name.endsWith('.llm')) {
                files.push(fullPath);
            }
        }
    }
    
    findFiles('.dx/markdown');
    
    for (const file of files) {
        const humanContent = fs.readFileSync(file, 'utf8');
        const llmContent = markdownWasm.human_to_llm(humanContent);
        fs.writeFileSync(file, llmContent, 'utf8');
        console.log(`  ✓ ${path.relative('.', file)}`);
        converted++;
    }
    
    return converted;
}

async function convertSerializerFiles() {
    console.log('\n📦 Converting Serializer .llm files...');
    
    let converted = 0;
    const files = [];
    
    function findFiles(dir) {
        const entries = fs.readdirSync(dir, { withFileTypes: true });
        for (const entry of entries) {
            const fullPath = path.join(dir, entry.name);
            if (entry.isDirectory()) {
                findFiles(fullPath);
            } else if (entry.name.endsWith('.llm')) {
                files.push(fullPath);
            }
        }
    }
    
    findFiles('.dx/serializer');
    
    for (const file of files) {
        const humanContent = fs.readFileSync(file, 'utf8');
        const llmContent = serializerWasm.human_to_llm(humanContent);
        fs.writeFileSync(file, llmContent, 'utf8');
        console.log(`  ✓ ${path.relative('.', file)}`);
        converted++;
    }
    
    return converted;
}

async function main() {
    console.log('🔄 Converting .llm files to LLM format...\n');
    
    const mdCount = await convertMarkdownFiles();
    const srCount = await convertSerializerFiles();
    
    console.log('\n✅ Conversion Complete!');
    console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    console.log(`Markdown files: ${mdCount}`);
    console.log(`Serializer files: ${srCount}`);
    console.log(`Total: ${mdCount + srCount}`);
}

main().catch(err => {
    console.error('❌ Error:', err);
    process.exit(1);
});
