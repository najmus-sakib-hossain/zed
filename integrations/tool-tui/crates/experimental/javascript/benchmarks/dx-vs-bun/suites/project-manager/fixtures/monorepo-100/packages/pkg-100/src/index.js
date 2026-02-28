/**
 * Package: pkg-100
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-099';
// import { something } from 'pkg-098';
// import { something } from 'pkg-097';

export function main() {
    console.log('pkg-100 main function');
    return { name: 'pkg-100', version: '1.0.0' };
}

export function helper100() {
    return 100 * 2;
}

export const config = {
    packageNumber: 100,
    totalPackages: 100
};
