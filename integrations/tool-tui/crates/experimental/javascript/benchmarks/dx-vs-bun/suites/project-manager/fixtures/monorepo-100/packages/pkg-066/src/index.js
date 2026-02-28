/**
 * Package: pkg-066
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-065';
// import { something } from 'pkg-064';
// import { something } from 'pkg-063';

export function main() {
    console.log('pkg-066 main function');
    return { name: 'pkg-066', version: '1.0.0' };
}

export function helper66() {
    return 66 * 2;
}

export const config = {
    packageNumber: 66,
    totalPackages: 100
};
