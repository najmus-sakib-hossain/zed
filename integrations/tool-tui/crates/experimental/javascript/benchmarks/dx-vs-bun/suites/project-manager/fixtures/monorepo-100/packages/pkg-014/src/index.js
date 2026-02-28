/**
 * Package: pkg-014
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-013';
// import { something } from 'pkg-012';
// import { something } from 'pkg-011';

export function main() {
    console.log('pkg-014 main function');
    return { name: 'pkg-014', version: '1.0.0' };
}

export function helper14() {
    return 14 * 2;
}

export const config = {
    packageNumber: 14,
    totalPackages: 100
};
