/**
 * Package: pkg-062
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-061';
// import { something } from 'pkg-060';
// import { something } from 'pkg-059';

export function main() {
    console.log('pkg-062 main function');
    return { name: 'pkg-062', version: '1.0.0' };
}

export function helper62() {
    return 62 * 2;
}

export const config = {
    packageNumber: 62,
    totalPackages: 100
};
