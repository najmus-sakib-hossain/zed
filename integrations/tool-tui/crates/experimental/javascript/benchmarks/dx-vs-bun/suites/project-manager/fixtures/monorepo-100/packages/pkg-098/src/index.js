/**
 * Package: pkg-098
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-097';
// import { something } from 'pkg-096';
// import { something } from 'pkg-095';

export function main() {
    console.log('pkg-098 main function');
    return { name: 'pkg-098', version: '1.0.0' };
}

export function helper98() {
    return 98 * 2;
}

export const config = {
    packageNumber: 98,
    totalPackages: 100
};
