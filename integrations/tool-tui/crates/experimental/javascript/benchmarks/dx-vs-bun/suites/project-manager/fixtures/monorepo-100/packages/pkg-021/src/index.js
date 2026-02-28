/**
 * Package: pkg-021
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-020';
// import { something } from 'pkg-019';
// import { something } from 'pkg-018';

export function main() {
    console.log('pkg-021 main function');
    return { name: 'pkg-021', version: '1.0.0' };
}

export function helper21() {
    return 21 * 2;
}

export const config = {
    packageNumber: 21,
    totalPackages: 100
};
