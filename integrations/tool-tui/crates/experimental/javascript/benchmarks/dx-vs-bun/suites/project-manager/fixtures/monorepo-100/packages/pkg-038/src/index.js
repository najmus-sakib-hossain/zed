/**
 * Package: pkg-038
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-037';
// import { something } from 'pkg-036';
// import { something } from 'pkg-035';

export function main() {
    console.log('pkg-038 main function');
    return { name: 'pkg-038', version: '1.0.0' };
}

export function helper38() {
    return 38 * 2;
}

export const config = {
    packageNumber: 38,
    totalPackages: 100
};
