/**
 * Package: pkg-046
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-045';
// import { something } from 'pkg-044';
// import { something } from 'pkg-043';

export function main() {
    console.log('pkg-046 main function');
    return { name: 'pkg-046', version: '1.0.0' };
}

export function helper46() {
    return 46 * 2;
}

export const config = {
    packageNumber: 46,
    totalPackages: 50
};
