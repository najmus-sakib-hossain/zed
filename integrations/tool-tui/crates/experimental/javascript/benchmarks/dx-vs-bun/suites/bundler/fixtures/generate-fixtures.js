// Generator script for medium and large bundler fixtures
const fs = require('fs');
const path = require('path');

function generateModule(index, category) {
    const funcName = `func${index}`;
    return `// Module ${index} - ${category}
export function ${funcName}(x) {
    return x * ${index} + ${Math.floor(Math.random() * 100)};
}

export function ${funcName}Async(x) {
    return Promise.resolve(${funcName}(x));
}

export const ${funcName}Const = ${index * 10};
`;
}

function generateIndex(modules, projectName) {
    let imports = '';
    let usage = '';

    modules.forEach((mod, i) => {
        imports += `import { func${i} } from './${mod}';\n`;
        usage += `  func${i}(${i}),\n`;
    });

    return `// Entry point for ${projectName}
${imports}
const results = [
${usage}];

console.log('Total modules:', ${modules.length});
console.log('Sum:', results.reduce((a, b) => a + b, 0));

export { results };
`;
}

function createProject(name, fileCount) {
    const projectDir = path.join(__dirname, name, 'src');
    fs.mkdirSync(projectDir, { recursive: true });

    const categories = ['utils', 'helpers', 'services', 'models', 'handlers'];
    const modules = [];

    // Create module files
    for (let i = 0; i < fileCount - 1; i++) {
        const category = categories[i % categories.length];
        const fileName = `module${i}.js`;
        const content = generateModule(i, category);
        fs.writeFileSync(path.join(projectDir, fileName), content);
        modules.push(`module${i}.js`);
    }

    // Create index file
    fs.writeFileSync(path.join(projectDir, 'index.js'), generateIndex(modules, name));

    // Create package.json
    fs.writeFileSync(path.join(__dirname, name, 'package.json'), JSON.stringify({
        name: `bundler-${name}`,
        version: '1.0.0',
        type: 'module',
        main: 'src/index.js'
    }, null, 4));

    console.log(`Created ${name} with ${fileCount} files`);
}

// Generate medium project (50 files)
createProject('medium-project', 50);

// Generate large project (150 files)
createProject('large-project', 150);

console.log('Done generating fixtures!');
