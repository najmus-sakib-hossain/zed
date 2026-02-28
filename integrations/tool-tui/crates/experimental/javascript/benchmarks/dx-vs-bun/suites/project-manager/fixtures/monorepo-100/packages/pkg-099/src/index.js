/**
 * Package: pkg-099
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-098';
// import { something } from 'pkg-097';
// import { something } from 'pkg-096';

export function main() {
    console.log('pkg-099 main function');
    return { name: 'pkg-099', version: '1.0.0' };
}

export function helper99() {
    return 99 * 2;
}

export const config = {
    packageNumber: 99,
    totalPackages: 100
};
