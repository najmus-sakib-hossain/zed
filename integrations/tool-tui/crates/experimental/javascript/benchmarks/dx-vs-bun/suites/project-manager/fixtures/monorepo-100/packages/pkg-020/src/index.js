/**
 * Package: pkg-020
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-019';
// import { something } from 'pkg-018';
// import { something } from 'pkg-017';

export function main() {
    console.log('pkg-020 main function');
    return { name: 'pkg-020', version: '1.0.0' };
}

export function helper20() {
    return 20 * 2;
}

export const config = {
    packageNumber: 20,
    totalPackages: 100
};
