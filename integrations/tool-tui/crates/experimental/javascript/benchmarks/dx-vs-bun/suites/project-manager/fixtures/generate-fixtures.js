/**
 * Generate monorepo fixtures for project manager benchmarks
 * Run with: node generate-fixtures.js
 */

const fs = require('fs');
const path = require('path');

function generatePackage(monorepoPath, pkgNum, totalPkgs) {
    const pkgName = `pkg-${String(pkgNum).padStart(3, '0')}`;
    const pkgDir = path.join(monorepoPath, 'packages', pkgName);

    // Create package directory
    fs.mkdirSync(pkgDir, { recursive: true });
    fs.mkdirSync(path.join(pkgDir, 'src'), { recursive: true });

    // Generate dependencies on previous packages (creates task graph)
    const deps = {};
    const numDeps = Math.min(pkgNum - 1, 3); // Max 3 deps per package
    for (let i = 0; i < numDeps; i++) {
        const depNum = Math.max(1, pkgNum - i - 1);
        const depName = `pkg-${String(depNum).padStart(3, '0')}`;
        deps[depName] = "1.0.0";
    }

    // Create package.json
    const packageJson = {
        name: pkgName,
        version: "1.0.0",
        main: "src/index.js",
        scripts: {
            build: "echo Building " + pkgName,
            test: "echo Testing " + pkgName,
            lint: "echo Linting " + pkgName
        },
        dependencies: deps
    };

    fs.writeFileSync(
        path.join(pkgDir, 'package.json'),
        JSON.stringify(packageJson, null, 2)
    );

    // Create source file
    const imports = Object.keys(deps).map(dep =>
        `// import { something } from '${dep}';`
    ).join('\n');

    const sourceCode = `/**
 * Package: ${pkgName}
 * Part of monorepo benchmark fixture
 */

${imports}

export function main() {
    console.log('${pkgName} main function');
    return { name: '${pkgName}', version: '1.0.0' };
}

export function helper${pkgNum}() {
    return ${pkgNum} * 2;
}

export const config = {
    packageNumber: ${pkgNum},
    totalPackages: ${totalPkgs}
};
`;

    fs.writeFileSync(path.join(pkgDir, 'src', 'index.js'), sourceCode);
}

function generateMonorepo(name, numPackages) {
    const monorepoPath = path.join(__dirname, name);
    const packagesDir = path.join(monorepoPath, 'packages');

    // Clean existing packages
    if (fs.existsSync(packagesDir)) {
        fs.rmSync(packagesDir, { recursive: true });
    }
    fs.mkdirSync(packagesDir, { recursive: true });

    console.log(`Generating ${name} with ${numPackages} packages...`);

    for (let i = 1; i <= numPackages; i++) {
        generatePackage(monorepoPath, i, numPackages);
        if (i % 10 === 0) {
            process.stdout.write(`  ${i}/${numPackages}\r`);
        }
    }

    console.log(`  ${numPackages}/${numPackages} - Done!`);
}

// Generate all monorepo fixtures
generateMonorepo('monorepo-10', 10);
generateMonorepo('monorepo-50', 50);
generateMonorepo('monorepo-100', 100);

console.log('All fixtures generated successfully!');
