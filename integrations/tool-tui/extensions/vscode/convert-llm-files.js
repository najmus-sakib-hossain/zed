#!/usr/bin/env node
/**
 * Convert all .llm files from human format to actual LLM format
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// Import the conversion functions (will be compiled from TypeScript)
const { parseHuman, serializeToLlm } = require('./out/humanParser');

function findLlmFiles(dir, files = []) {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    
    for (const entry of entries) {
        const fullPath = path.join(dir, entry.name);
        
        if (entry.isDirectory()) {
            if (entry.name !== 'node_modules' && entry.name !== 'target' && entry.name !== 'trash') {
                findLlmFiles(fullPath, files);
            }
        } else if (entry.isFile() && entry.name.endsWith('.llm')) {
            files.push(fullPath);
        }
    }
    
    return files;
}

function convertFile(filePath) {
    try {
        const content = fs.readFileSync(filePath, 'utf8');
        
        // Parse as human format
        const parseResult = parseHuman(content);
        
        if (!parseResult.success || !parseResult.document) {
            console.log(`  ⚠️  Failed to parse: ${filePath}`);
            return false;
        }
        
        // Convert to LLM format
        const llmContent = serializeToLlm(parseResult.document);
        
        // Write back
        fs.writeFileSync(filePath, llmContent, 'utf8');
        
        return true;
    } catch (error) {
        console.log(`  ❌ Error converting ${filePath}: ${error.message}`);
        return false;
    }
}

async function main() {
    console.log('🔄 Converting .llm files to LLM format...\n');
    
    // Compile TypeScript first
    console.log('📦 Compiling TypeScript...');
    try {
        execSync('npm run compile', { cwd: __dirname, stdio: 'inherit' });
    } catch (error) {
        console.error('❌ Failed to compile TypeScript');
        process.exit(1);
    }
    
    console.log('\n📝 Finding .llm files...');
    const dxDir = path.join(__dirname, '..', '.dx');
    const llmFiles = findLlmFiles(dxDir);
    
    console.log(`Found ${llmFiles.length} .llm files\n`);
    
    let converted = 0;
    let skipped = 0;
    
    for (const file of llmFiles) {
        const relativePath = path.relative(path.join(__dirname, '..'), file);
        process.stdout.write(`  Converting: ${relativePath}...`);
        
        if (convertFile(file)) {
            console.log(' ✓');
            converted++;
        } else {
            console.log(' ⚠️');
            skipped++;
        }
    }
    
    console.log('\n✅ Conversion Complete!');
    console.log(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
    console.log(`Converted: ${converted}`);
    console.log(`Skipped: ${skipped}`);
    console.log(`Total: ${llmFiles.length}`);
}

main().catch(error => {
    console.error('❌ Fatal error:', error);
    process.exit(1);
});
