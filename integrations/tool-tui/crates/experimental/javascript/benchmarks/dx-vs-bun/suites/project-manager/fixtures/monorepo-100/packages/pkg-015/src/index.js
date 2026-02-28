/**
 * Package: pkg-015
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-014';
// import { something } from 'pkg-013';
// import { something } from 'pkg-012';

export function main() {
    console.log('pkg-015 main function');
    return { name: 'pkg-015', version: '1.0.0' };
}

export function helper15() {
    return 15 * 2;
}

export const config = {
    packageNumber: 15,
    totalPackages: 100
};
