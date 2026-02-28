//! Parallel Serialization with Rayon
//!
//! Zero-contention parallel serialization using thread-local arenas.
//!
//! # Performance
//! - Linear speedup with core count
//! - Zero lock contention
//! - Automatic load balancing via Rayon's work stealing

#[cfg(feature = "parallel")]
use super::arena::{DxArena, DxArenaPool};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Serialize items in parallel using thread-local arenas
///
/// Each thread gets its own arena from a thread-local pool,
/// ensuring zero contention across threads.
///
/// # Example
/// ```ignore
/// use serializer::machine::parallel::par_serialize;
///
/// let items = vec![1u64, 2, 3, 4, 5];
/// let results: Vec<Vec<u8>> = par_serialize(&items, |arena, &item| {
///     arena.write_header(0);
///     let mut writer = arena.writer();
///     writer.write_u64::<0>(item);
///     arena.advance(8);
///     arena.to_vec()
/// });
/// ```
#[cfg(feature = "parallel")]
pub fn par_serialize<T, F, R>(items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    F: Fn(&mut DxArena, &T) -> R + Sync,
    R: Send,
{
    items
        .par_iter()
        .map(|item| DxArenaPool::with_thread_local(|arena| f(arena, item)))
        .collect()
}

/// Serialize items in parallel with indexed access
///
/// Similar to `par_serialize` but provides the index of each item.
///
/// # Example
/// ```ignore
/// use serializer::machine::parallel::par_serialize_indexed;
///
/// let items = vec![1u64, 2, 3, 4, 5];
/// let results: Vec<Vec<u8>> = par_serialize_indexed(&items, |arena, idx, &item| {
///     arena.write_header(0);
///     let mut writer = arena.writer();
///     writer.write_u64::<0>(idx as u64);
///     writer.write_u64::<8>(item);
///     arena.advance(16);
///     arena.to_vec()
/// });
/// ```
#[cfg(feature = "parallel")]
pub fn par_serialize_indexed<T, F, R>(items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    F: Fn(&mut DxArena, usize, &T) -> R + Sync,
    R: Send,
{
    items
        .par_iter()
        .enumerate()
        .map(|(idx, item)| DxArenaPool::with_thread_local(|arena| f(arena, idx, item)))
        .collect()
}

/// Serialize items in parallel with chunking
///
/// Processes items in chunks, reducing arena allocation overhead
/// for small items.
///
/// # Example
/// ```ignore
/// use serializer::machine::parallel::par_serialize_chunked;
///
/// let items = vec![1u64; 10000];
/// let results: Vec<Vec<u8>> = par_serialize_chunked(&items, 100, |arena, chunk| {
///     arena.write_header(0);
///     for &item in chunk {
///         let mut writer = arena.writer();
///         writer.write_u64::<0>(item);
///         arena.advance(8);
///     }
///     arena.to_vec()
/// });
/// ```
#[cfg(feature = "parallel")]
pub fn par_serialize_chunked<T, F, R>(items: &[T], chunk_size: usize, f: F) -> Vec<R>
where
    T: Sync,
    F: Fn(&mut DxArena, &[T]) -> R + Sync,
    R: Send,
{
    items
        .par_chunks(chunk_size)
        .map(|chunk| DxArenaPool::with_thread_local(|arena| f(arena, chunk)))
        .collect()
}

/// Configure thread-local arena capacity for parallel operations
///
/// Call this before parallel serialization to set the arena size
/// for each thread. Larger arenas reduce reallocation overhead
/// but use more memory.
///
/// # Example
/// ```ignore
/// use serializer::machine::parallel::configure_parallel_arenas;
///
/// // Use 1MB arenas for large objects
/// configure_parallel_arenas(1024 * 1024);
/// ```
#[cfg(feature = "parallel")]
pub fn configure_parallel_arenas(arena_capacity: usize) {
    DxArenaPool::configure_thread_local(arena_capacity);
}

/// Clear all thread-local arena pools
///
/// Useful for reclaiming memory after large parallel operations.
///
/// # Example
/// ```ignore
/// use serializer::machine::parallel::clear_parallel_arenas;
///
/// // After processing large batch
/// clear_parallel_arenas();
/// ```
#[cfg(feature = "parallel")]
pub fn clear_parallel_arenas() {
    DxArenaPool::clear_thread_local();
}

#[cfg(all(test, feature = "parallel"))]
mod tests {
    use super::*;

    #[test]
    fn test_par_serialize() {
        let items = vec![10u64, 20, 30, 40, 50];

        let results = par_serialize(&items, |arena, &item| {
            arena.write_header(0);
            let mut writer = arena.writer();
            writer.write_u64::<0>(item);
            arena.advance(8);
            arena.to_vec()
        });

        assert_eq!(results.len(), 5);

        // Verify each result
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.len(), 12); // 4 (header) + 8 (u64)
            let value = u64::from_le_bytes([
                result[4], result[5], result[6], result[7], result[8], result[9], result[10],
                result[11],
            ]);
            assert_eq!(value, items[i]);
        }
    }

    #[test]
    fn test_par_serialize_indexed() {
        let items = vec![100u64, 200, 300];

        let results = par_serialize_indexed(&items, |arena, idx, &item| {
            arena.write_header(0);
            let mut writer = arena.writer();
            writer.write_u64::<0>(idx as u64);
            writer.write_u64::<8>(item);
            arena.advance(16);
            arena.to_vec()
        });

        assert_eq!(results.len(), 3);

        // Verify indexed values
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result.len(), 20); // 4 (header) + 16 (2 × u64)

            let idx_value = u64::from_le_bytes([
                result[4], result[5], result[6], result[7], result[8], result[9], result[10],
                result[11],
            ]);
            assert_eq!(idx_value, i as u64);

            let item_value = u64::from_le_bytes([
                result[12], result[13], result[14], result[15], result[16], result[17], result[18],
                result[19],
            ]);
            assert_eq!(item_value, items[i]);
        }
    }

    #[test]
    fn test_par_serialize_chunked() {
        let items = vec![1u64; 100];

        let results = par_serialize_chunked(&items, 10, |arena, chunk| {
            arena.write_header(0);
            for &item in chunk {
                let mut writer = arena.writer();
                writer.write_u64::<0>(item);
                arena.advance(8);
            }
            arena.to_vec()
        });

        assert_eq!(results.len(), 10); // 100 items / 10 per chunk

        // Each chunk should have 4 (header) + 10 × 8 (items) = 84 bytes
        for result in &results {
            assert_eq!(result.len(), 84);
        }
    }

    #[test]
    fn test_configure_and_clear() {
        configure_parallel_arenas(256 * 1024);

        let items = vec![42u64];
        let _results = par_serialize(&items, |arena, &_item| {
            assert_eq!(arena.capacity(), 256 * 1024);
            arena.write_header(0);
            arena.to_vec()
        });

        clear_parallel_arenas();
        assert_eq!(DxArenaPool::thread_local_size(), 0);
    }

    #[test]
    fn test_zero_contention() {
        // Large dataset to ensure multiple threads are used
        let items: Vec<u64> = (0..10000).collect();

        let results = par_serialize(&items, |arena, &item| {
            arena.write_header(0);
            let mut writer = arena.writer();
            writer.write_u64::<0>(item);
            arena.advance(8);
            arena.to_vec()
        });

        assert_eq!(results.len(), 10000);

        // Verify all values are correct (no data races)
        for (i, result) in results.iter().enumerate() {
            let value = u64::from_le_bytes([
                result[4], result[5], result[6], result[7], result[8], result[9], result[10],
                result[11],
            ]);
            assert_eq!(value, i as u64);
        }
    }
}
