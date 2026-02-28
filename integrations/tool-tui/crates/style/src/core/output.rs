use memmap2::MmapMut;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{Duration, Instant};

static mut MMAP_THRESHOLD_BYTES: u64 = 64 * 1024;

const MANAGED_MARKER: &str = "/*
  ████████   ██             ██
 ██░░░░░░   ░██    ██   ██ ░██
░██        ██████ ░░██ ██  ░██  █████
░█████████░░░██░   ░░███   ░██ ██░░░██
░░░░░░░░██  ░██     ░██    ░██░███████
       ░██  ░██     ██     ░██░██░░░░
 ████████   ░░██   ██      ███░░██████
░░░░░░░░     ░░   ░░      ░░░  ░░░░░░
Dx Style v0.0.1 | MIT License | https://dx.com
*/\n";
const MANAGED_MARKER_PREFIX: &str = "/*
  ████████   ██             ██
 ██░░░░░░   ░██    ██   ██ ░██
░██        ██████ ░░██ ██  ░██  █████
░█████████░░░██░   ░░███   ░██ ██░░░██
░░░░░░░░██  ░██     ░██    ░██░███████
       ░██  ░██     ██     ░██░██░░░░
 ████████   ░░██   ██      ███░░██████
░░░░░░░░     ░░   ░░      ░░░  ░░░░░░
Dx Style v0.0.1 | MIT License | https://dx.com
*/\n";

pub enum CssBackend {
    Writer {
        writer: BufWriter<File>,
        logical_len: usize,
        dirty: bool,
        last_flush: Instant,
    },
    Mmap {
        file: File,
        mmap: MmapMut,
        logical_len: usize,
        dirty: bool,
        last_flush: Instant,
    },
}

pub struct CssOutput {
    backend: CssBackend,
    managed_base: usize,
    path: String,
}

impl CssOutput {
    pub fn open(path: &str) -> std::io::Result<Self> {
        let p = Path::new(path);
        if !p.exists() {
            File::create(p)?;
        }
        let meta_len = p.metadata().map(|m| m.len()).unwrap_or(0);
        let threshold = unsafe { MMAP_THRESHOLD_BYTES };
        if meta_len >= threshold {
            Self::open_mmap(path)
        } else {
            Self::open_writer(path)
        }
    }

    fn ensure_marker_in_memory(buf: &[u8]) -> Option<usize> {
        twoway::find_bytes(buf, MANAGED_MARKER.as_bytes()).map(|pos| pos + MANAGED_MARKER.len())
    }

    fn open_writer(path: &str) -> std::io::Result<Self> {
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        let mut existing = Vec::new();
        f.read_to_end(&mut existing)?;
        if !existing.is_empty() && Self::ensure_marker_in_memory(&existing).is_none() {
            if let Some(prefix_pos) =
                twoway::find_bytes(&existing, MANAGED_MARKER_PREFIX.as_bytes())
            {
                let after = &existing[prefix_pos..];
                let has_close = after.windows(2).position(|w| w == b"*/");
                if has_close.is_none() {
                    let mut repaired = existing[..prefix_pos].to_vec();
                    repaired.extend_from_slice(MANAGED_MARKER.as_bytes());
                    if let Some(nl) = after.iter().position(|b| *b == b'\n') {
                        repaired.extend_from_slice(&after[nl + 1..]);
                    }
                    existing = repaired;
                    f.set_len(0)?;
                    f.seek(SeekFrom::Start(0))?;
                    f.write_all(&existing)?;
                    f.flush()?;
                }
            }
        }
        if Self::ensure_marker_in_memory(&existing).is_none() {
            if let Some(first_non_zero) = existing.iter().position(|b| *b != 0) {
                if first_non_zero > 0 {
                    let mut trimmed = existing.split_off(first_non_zero);
                    std::mem::swap(&mut existing, &mut trimmed);
                    f.set_len(0)?;
                    f.seek(SeekFrom::Start(0))?;
                    f.write_all(&existing)?;
                    f.flush()?;
                }
            } else if !existing.is_empty() {
                existing.clear();
                f.set_len(0)?;
            }
        }
        let mut logical_len = existing.len();
        let managed_base = if let Some(base) = Self::ensure_marker_in_memory(&existing) {
            base
        } else {
            f.seek(SeekFrom::End(0))?;
            f.write_all(MANAGED_MARKER.as_bytes())?;
            logical_len += MANAGED_MARKER.len();
            logical_len
        };
        Ok(Self {
            backend: CssBackend::Writer {
                writer: BufWriter::with_capacity(64 * 1024, f),
                logical_len,
                dirty: false,
                last_flush: Instant::now(),
            },
            managed_base,
            path: path.to_string(),
        })
    }

    fn open_mmap(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        let mut was_empty = false;
        if file.metadata()?.len() == 0 {
            file.set_len(4096)?;
            was_empty = true;
        }
        let mut logical_len = if was_empty {
            0
        } else {
            file.metadata()?.len() as usize
        };
        let mut tmp = Vec::with_capacity(logical_len);
        {
            let mut reader = std::io::BufReader::new(&file);
            use std::io::Read;
            reader.read_to_end(&mut tmp)?;
        }
        let all_zeros = !tmp.is_empty() && tmp.iter().all(|b| *b == 0);
        if all_zeros {
            logical_len = 0;
        }
        if !all_zeros && !tmp.is_empty() {
            if let Some(first_non_zero) = tmp.iter().position(|b| *b != 0) {
                if first_non_zero > 0 {
                    let new_len = tmp.len() - first_non_zero;
                    let mut new_buf = Vec::with_capacity(new_len);
                    new_buf.extend_from_slice(&tmp[first_non_zero..]);
                    file.set_len(new_len as u64)?;
                    let cap = (new_len.next_power_of_two()).max(4096) as u64;
                    if cap > new_len as u64 {
                        file.set_len(cap)?;
                    }
                    let mut mmap_temp = unsafe { MmapMut::map_mut(&file)? };
                    mmap_temp[..new_len].copy_from_slice(&new_buf);
                    mmap_temp.flush()?;
                    tmp = new_buf;
                    logical_len = new_len;
                }
            }
        }
        if logical_len > 0 && Self::ensure_marker_in_memory(&tmp).is_none() {
            if let Some(prefix_pos) = twoway::find_bytes(&tmp, MANAGED_MARKER_PREFIX.as_bytes()) {
                let after = &tmp[prefix_pos..];
                let has_close = after.windows(2).position(|w| w == b"*/");
                if has_close.is_none() {
                    let mut repaired = tmp[..prefix_pos].to_vec();
                    repaired.extend_from_slice(MANAGED_MARKER.as_bytes());
                    if let Some(nl) = after.iter().position(|b| *b == b'\n') {
                        repaired.extend_from_slice(&after[nl + 1..]);
                    }
                    let needed = repaired.len();
                    if needed > file.metadata()?.len() as usize {
                        file.set_len((needed.next_power_of_two()).max(4096) as u64)?;
                    }
                    let mut mmap_temp = unsafe { MmapMut::map_mut(&file)? };
                    mmap_temp[..repaired.len()].copy_from_slice(&repaired);
                    mmap_temp.flush()?;
                    logical_len = repaired.len();
                    tmp = repaired;
                }
            }
        }
        let managed_base = if let Some(base) = if logical_len > 0 {
            Self::ensure_marker_in_memory(&tmp)
        } else {
            None
        } {
            base
        } else {
            let place_at = if logical_len == 0 { 0 } else { logical_len };
            let needed = place_at + MANAGED_MARKER.len();
            if needed > file.metadata()?.len() as usize {
                let new_len = (needed.next_power_of_two()).max(4096) as u64;
                file.set_len(new_len)?;
            }
            let mut mmap_temp = unsafe { MmapMut::map_mut(&file)? };
            mmap_temp[place_at..place_at + MANAGED_MARKER.len()]
                .copy_from_slice(MANAGED_MARKER.as_bytes());
            mmap_temp.flush()?;
            logical_len = place_at + MANAGED_MARKER.len();
            logical_len
        };
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        Ok(Self {
            backend: CssBackend::Mmap {
                file,
                mmap,
                logical_len,
                dirty: false,
                last_flush: Instant::now(),
            },
            managed_base,
            path: path.to_string(),
        })
    }

    pub fn replace(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        match &mut self.backend {
            CssBackend::Writer {
                writer,
                logical_len,
                dirty,
                ..
            } => {
                let truncate_len = self.managed_base as u64;
                writer.get_mut().set_len(truncate_len)?;
                writer.seek(SeekFrom::Start(truncate_len))?;
                writer.write_all(bytes)?;
                *logical_len = self.managed_base + bytes.len();
                *dirty = true;
            }
            CssBackend::Mmap {
                file,
                mmap,
                logical_len,
                dirty,
                ..
            } => {
                let needed_total = self.managed_base + bytes.len();
                if mmap.len() < needed_total {
                    let new_len = (needed_total.next_power_of_two()).max(4096);
                    file.set_len(new_len as u64)?;
                    *mmap = unsafe { MmapMut::map_mut(&*file)? };
                }
                mmap[self.managed_base..self.managed_base + bytes.len()].copy_from_slice(bytes);
                *logical_len = needed_total;
                file.set_len(*logical_len as u64)?;
                *dirty = true;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        match &mut self.backend {
            CssBackend::Writer {
                writer,
                logical_len,
                dirty,
                ..
            } => {
                writer.seek(SeekFrom::Start(*logical_len as u64))?;
                writer.write_all(bytes)?;
                *logical_len += bytes.len();
                *dirty = true;
            }
            CssBackend::Mmap {
                file,
                mmap,
                logical_len,
                dirty,
                ..
            } => {
                let needed = *logical_len + bytes.len();
                if mmap.len() < needed {
                    let new_len = (needed.next_power_of_two()).max(4096);
                    file.set_len(new_len as u64)?;
                    *mmap = unsafe { MmapMut::map_mut(&*file)? };
                }
                mmap[*logical_len..needed].copy_from_slice(bytes);
                *logical_len = needed;
                *dirty = true;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn flush_if_dirty(&mut self) -> std::io::Result<()> {
        use std::sync::OnceLock;
        static FLUSH_INTERVAL: OnceLock<Duration> = OnceLock::new();
        let interval = *FLUSH_INTERVAL.get_or_init(|| {
            std::env::var("DX_FLUSH_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_millis)
                .unwrap_or_else(|| Duration::from_millis(25))
        });
        match &mut self.backend {
            CssBackend::Writer {
                writer,
                dirty,
                last_flush,
                ..
            } => {
                let should_flush = cfg!(feature = "eager-flush")
                    || interval.is_zero()
                    || (*dirty && last_flush.elapsed() >= interval);
                if should_flush {
                    writer.flush()?;
                    *dirty = false;
                    *last_flush = Instant::now();
                }
            }
            CssBackend::Mmap {
                mmap,
                dirty,
                last_flush,
                ..
            } => {
                let should_flush = cfg!(feature = "eager-flush")
                    || interval.is_zero()
                    || (*dirty && last_flush.elapsed() >= interval);
                if should_flush {
                    mmap.flush()?;
                    *dirty = false;
                    *last_flush = Instant::now();
                }
            }
        }
        Ok(())
    }

    pub fn flush_now(&mut self) -> std::io::Result<()> {
        match &mut self.backend {
            CssBackend::Writer {
                writer,
                dirty,
                last_flush,
                ..
            } => {
                if *dirty {
                    writer.flush()?;
                    *dirty = false;
                }
                *last_flush = Instant::now();
            }
            CssBackend::Mmap {
                mmap,
                dirty,
                last_flush,
                ..
            } => {
                if *dirty {
                    mmap.flush()?;
                    *dirty = false;
                }
                *last_flush = Instant::now();
            }
        }
        Ok(())
    }

    pub fn append_inside_final_block(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.is_empty() {
            return Ok(self.current_len());
        }
        match &mut self.backend {
            CssBackend::Writer {
                writer,
                logical_len,
                dirty,
                ..
            } => {
                if *logical_len < 2 {
                    return Err(std::io::Error::other("file too short"));
                }
                let insert_pos = *logical_len - 2;
                writer.seek(SeekFrom::Start(insert_pos as u64))?;
                writer.write_all(bytes)?;
                writer.write_all(b"}\n")?;
                *logical_len = insert_pos + bytes.len() + 2;
                *dirty = true;
                Ok(insert_pos - self.managed_base)
            }
            CssBackend::Mmap {
                file,
                mmap,
                logical_len,
                dirty,
                ..
            } => {
                if *logical_len < 2 {
                    return Err(std::io::Error::other("file too short"));
                }
                let insert_pos = *logical_len - 2;
                let needed = insert_pos + bytes.len() + 2;
                if mmap.len() < needed {
                    let new_len = (needed.next_power_of_two()).max(4096);
                    file.set_len(new_len as u64)?;
                    *mmap = unsafe { MmapMut::map_mut(&*file)? };
                }
                mmap[insert_pos..insert_pos + bytes.len()].copy_from_slice(bytes);
                mmap[insert_pos + bytes.len()..insert_pos + bytes.len() + 2]
                    .copy_from_slice(b"}\n");
                *logical_len = insert_pos + bytes.len() + 2;
                *dirty = true;
                Ok(insert_pos - self.managed_base)
            }
        }
    }

    #[allow(dead_code)]
    pub fn truncate_managed_to(&mut self, rel_len: usize) -> std::io::Result<()> {
        match &mut self.backend {
            CssBackend::Writer {
                writer,
                logical_len,
                ..
            } => {
                let new_total = self.managed_base + rel_len;
                writer.get_mut().set_len(new_total as u64)?;
                writer.seek(SeekFrom::Start(new_total as u64))?;
                *logical_len = new_total;
            }
            CssBackend::Mmap {
                file,
                mmap,
                logical_len,
                ..
            } => {
                let new_total = self.managed_base + rel_len;
                file.set_len(new_total as u64)?;
                *mmap = unsafe { MmapMut::map_mut(&*file)? };
                *logical_len = new_total;
            }
        }
        Ok(())
    }

    pub fn current_len(&self) -> usize {
        let total = match &self.backend {
            CssBackend::Writer { logical_len, .. } => *logical_len,
            CssBackend::Mmap { logical_len, .. } => *logical_len,
        };
        total.saturating_sub(self.managed_base)
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn blank_range(&mut self, start: usize, len: usize) -> std::io::Result<()> {
        if len == 0 {
            return Ok(());
        }
        let abs_start = self.managed_base + start;
        match &mut self.backend {
            CssBackend::Writer {
                writer,
                logical_len,
                dirty,
                ..
            } => {
                if abs_start + len > *logical_len {
                    return Ok(());
                }
                writer.flush()?;
                let space_len = if len > 0 { len - 1 } else { 0 };
                writer.seek(SeekFrom::Start(abs_start as u64))?;
                const SPACE_BLOCK: [u8; 1024] = [b' '; 1024];
                let mut remaining = space_len;
                while remaining > 0 {
                    let chunk = remaining.min(SPACE_BLOCK.len());
                    writer.write_all(&SPACE_BLOCK[..chunk])?;
                    remaining -= chunk;
                }
                if len > 0 {
                    writer.write_all(b"\n")?;
                }
                writer.seek(SeekFrom::Start(*logical_len as u64))?;
                *dirty = true;
            }
            CssBackend::Mmap {
                mmap,
                logical_len,
                dirty,
                ..
            } => {
                if abs_start + len > *logical_len {
                    return Ok(());
                }
                let space_len = if len > 0 { len - 1 } else { 0 };
                for b in &mut mmap[abs_start..abs_start + space_len] {
                    *b = b' ';
                }
                if len > 0 {
                    mmap[abs_start + len - 1] = b'\n';
                }
                *dirty = true;
            }
        }
        Ok(())
    }
}

impl Drop for CssOutput {
    fn drop(&mut self) {
        match &mut self.backend {
            CssBackend::Writer { writer, dirty, .. } => {
                if *dirty {
                    let _ = writer.flush();
                }
            }
            CssBackend::Mmap { mmap, dirty, .. } => {
                if *dirty {
                    let _ = mmap.flush();
                }
            }
        }
    }
}

pub fn set_mmap_threshold(bytes: u64) {
    unsafe {
        MMAP_THRESHOLD_BYTES = bytes;
    }
}
