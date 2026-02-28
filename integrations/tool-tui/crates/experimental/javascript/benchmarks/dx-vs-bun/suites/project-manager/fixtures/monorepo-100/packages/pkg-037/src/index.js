/**
 * Package: pkg-037
 * Part of monorepo benchmark fixture
 */

// import { something } from 'pkg-036';
// import { something } from 'pkg-035';
// import { something } from 'pkg-034';

export function main() {
    console.log('pkg-037 main function');
    return { name: 'pkg-037', version: '1.0.0' };
}

export function helper37() {
    return 37 * 2;
}

export const config = {
    packageNumber: 37,
    totalPackages: 100
};
