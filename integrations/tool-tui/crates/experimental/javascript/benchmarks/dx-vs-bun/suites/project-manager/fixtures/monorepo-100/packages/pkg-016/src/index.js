/**
 * Package: pkg-016
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-015';
// import { something } from 'pkg-014';
// import { something } from 'pkg-013';

export function main() {
    console.log('pkg-016 main function');
    return { name: 'pkg-016', version: '1.0.0' };
}

export function helper16() {
    return 16 * 2;
}

export const config = {
    packageNumber: 16,
    totalPackages: 100
};
