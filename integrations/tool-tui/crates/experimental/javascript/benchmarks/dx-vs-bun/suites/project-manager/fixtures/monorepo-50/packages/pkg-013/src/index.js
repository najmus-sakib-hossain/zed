/**
 * Package: pkg-013
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-012';
// import { something } from 'pkg-011';
// import { something } from 'pkg-010';

export function main() {
    console.log('pkg-013 main function');
    return { name: 'pkg-013', version: '1.0.0' };
}

export function helper13() {
    return 13 * 2;
}

export const config = {
    packageNumber: 13,
    totalPackages: 50
};
