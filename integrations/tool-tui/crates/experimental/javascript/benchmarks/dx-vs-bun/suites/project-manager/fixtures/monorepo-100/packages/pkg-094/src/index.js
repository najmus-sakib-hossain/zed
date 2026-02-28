/**
 * Package: pkg-094
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-093';
// import { something } from 'pkg-092';
// import { something } from 'pkg-091';

export function main() {
    console.log('pkg-094 main function');
    return { name: 'pkg-094', version: '1.0.0' };
}

export function helper94() {
    return 94 * 2;
}

export const config = {
    packageNumber: 94,
    totalPackages: 100
};
