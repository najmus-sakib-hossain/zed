const fs = require('fs');
const path = require('path');

// Import the conversion function
const conversionPath = path.join(__dirname, 'out', 'conversion.js');
const { llmToHumanFallback } = require(conversionPath);

const testFiles = [
    'example1_mixed.sr',
    'example2_nested.sr',
    'example3_deep.sr',
    'example4_config.sr',
    'example5_leaf.sr'
];

console.log('Testing TypeScript conversion against Rust output...\n');

let allMatch = true;

for (const file of testFiles) {
    const srPath = path.join(__dirname, '..', 'essence', file);
    const rustHumanPath = path.join(__dirname, '..', '.dx', 'serializer', 'essence', file.replace('.sr', '.human'));
    
    // Read SR file
    const llmContent = fs.readFileSync(srPath, 'utf-8');
    
    // Convert using TypeScript
    const tsHuman = llmToHumanFallback(llmContent);
    
    // Read Rust output
    const rustHuman = fs.readFileSync(rustHumanPath, 'utf-8');
    
    // Compare
    const match = tsHuman.trim() === rustHuman.trim();
    
    console.log(`${file}: ${match ? '✅ MATCH' : '❌ MISMATCH'}`);
    
    if (!match) {
        allMatch = false;
        console.log('\n--- TypeScript Output ---');
        console.log(tsHuman.substring(0, 500));
        console.log('\n--- Rust Output ---');
        console.log(rustHuman.substring(0, 500));
        console.log('\n');
    }
}

console.log(`\n${allMatch ? '✅ All conversions match!' : '❌ Some conversions differ'}`);
process.exit(allMatch ? 0 : 1);
