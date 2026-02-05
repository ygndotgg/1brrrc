#![feature(portable_simd)]
#![feature(cold_path)]
use std::{
    borrow::Borrow,
    collections::{BTreeMap, HashMap, btree_map::Entry},
    fs::File,
    hash::{BuildHasher, Hasher},
    io::Write,
    mem::ManuallyDrop,
    os::fd::AsRawFd,
    ptr,
    simd::{Simd, cmp::SimdPartialEq},
    thread::available_parallelism,
};
unsafe impl Send for StrVec {}

use libc::{MADV_SEQUENTIAL, MAP_PRIVATE, PROT_READ, madvise};

fn main() {
    let f = File::open("measurements.txt").unwrap();
    let mut stats = BTreeMap::new();
    std::thread::scope(|scope| {
        let map = mmap(&f);
        let nthreads = available_parallelism().unwrap();

        let (tx, rx) = std::sync::mpsc::sync_channel(nthreads.get());
        let chunk_len = map.len() / nthreads;
        let mut at = 0;

        for _ in 0..nthreads.get() {
            let start = at;
            let end = (at + chunk_len).min(map.len());
            let end = if end == map.len() {
                map.len()
            } else {
                let newline_at = find_newline(&map[end..]).unwrap();
                end + newline_at + 1
            };
            let map = &map[start..end];
            at = end;
            let tx = tx.clone();
            scope.spawn(move || tx.send(abra_kadabra(map)));
        }
        drop(tx);
        for one_stat in rx {
            for (k, v) in one_stat {
                // SAFETY: the README promised
                match stats.entry(unsafe { String::from_utf8_unchecked(k.as_ref().to_vec()) }) {
                    Entry::Vacant(none) => {
                        none.insert(v);
                    }
                    Entry::Occupied(some) => {
                        let stat = some.into_mut();
                        stat.min = stat.min.min(v.min);
                        stat.sum += v.sum;
                        stat.count += v.count;
                        stat.max = stat.max.max(v.max);
                    }
                }
            }
        }
    });

    print(stats);
}

const INLINE: usize = std::mem::size_of::<AllocatedStrVec>();
const LAST: usize = INLINE - 1;

#[repr(C)]
union StrVec {
    inlined: [u8; INLINE],
    heap: ManuallyDrop<AllocatedStrVec>,
}

impl StrVec {
    pub fn new(s: &[u8]) -> Self {
        if s.len() < INLINE {
            let mut combined = [0u8; INLINE];
            combined[..s.len()].copy_from_slice(s);
            combined[LAST] = s.len() as u8 + 1;
            Self { inlined: combined }
        } else {
            let ptr = Box::into_raw(s.to_vec().into_boxed_slice());
            Self {
                heap: ManuallyDrop::new(AllocatedStrVec {
                    ptr: ptr.cast(),
                    len: ptr.len().to_le(),
                }),
            }
        }
    }
}

impl Drop for StrVec {
    fn drop(&mut self) {
        unsafe {
            if self.inlined[LAST] == 0x00 {
                ManuallyDrop::drop(&mut self.heap);
            }
        }
    }
}

impl AsRef<[u8]> for StrVec {
    fn as_ref(&self) -> &[u8] {
        unsafe {
            if self.inlined[LAST] != 0x00 {
                let len = self.inlined[LAST] as usize - 1;
                std::slice::from_raw_parts(self.inlined.as_ptr(), len)
            } else {
                std::hint::cold_path();
                let len = usize::from_le(self.heap.len);
                let ptr = self.heap.ptr;
                std::slice::from_raw_parts(ptr, len)
            }
        }
    }
}

impl Borrow<[u8]> for StrVec {
    fn borrow(&self) -> &[u8] {
        self.as_ref()
    }
}

#[derive(Copy, Clone)]
struct Stat {
    min: i16,
    max: i16,
    sum: i64,
    count: u32,
}

impl Default for Stat {
    fn default() -> Self {
        Self {
            min: i16::MAX,
            sum: 0,
            count: 0,
            max: i16::MIN,
        }
    }
}

impl PartialEq for StrVec {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            self.inlined[LAST] == other.inlined[LAST] && {
                std::hint::cold_path();
                self.as_ref() == other.as_ref()
            }
        }
    }
}

impl Eq for StrVec {}
impl std::hash::Hash for StrVec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

#[repr(C)]
struct AllocatedStrVec {
    ptr: *mut u8,
    len: usize,
}

const HASH_K: u64 = 0xf1357aea2e62a9c5;
const HASH_SEED: u64 = 0x13198a2e03707344;
struct Fasthasher(u64);

struct FasthasherBuilder;

impl BuildHasher for FasthasherBuilder {
    type Hasher = Fasthasher;
    fn build_hasher(&self) -> Self::Hasher {
        Fasthasher(0)
    }
}

impl Hasher for Fasthasher {
    fn finish(&self) -> u64 {
        self.0.rotate_left(26)
    }
    fn write(&mut self, bytes: &[u8]) {
        let len = bytes.len();
        let mut acc = HASH_SEED;
        match len {
            0..4 => {
                let low = bytes[0];
                let mid = bytes[len / 2];
                let high = bytes[len - 1];
                acc ^= (low as u64) | ((mid as u64) << 8) | ((high as u64) << 16);
            }
            4.. => {
                acc ^= u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as u64;
            }
        }
        self.0 = self.0.wrapping_add(acc).wrapping_mul(HASH_K);
    }
}

#[inline(never)]
fn abra_kadabra(k: &[u8]) -> HashMap<StrVec, Stat, FasthasherBuilder> {
    let mut stats = HashMap::with_capacity_and_hasher(1024, FasthasherBuilder);
    let mut at = 0;
    while at < k.len() {
        let new_line = at + unsafe { find_newline(&k[at..]).unwrap_unchecked() };
        let line = unsafe { k.get_unchecked(at..new_line) };
        at = new_line + 1;
        let (station, temp) = split_semi(line);
        let temp = parse_temperature(temp);
        update_stats(&mut stats, station, temp);
    }

    stats
}

fn update_stats(stats: &mut HashMap<StrVec, Stat, FasthasherBuilder>, station: &[u8], t: i16) {
    let stat = match stats.get_mut(station) {
        Some(stats) => stats,
        None => stats.entry(StrVec::new(station)).or_default(),
    };
    if t < stat.min {
        stat.min = t;
    }
    if t > stat.max {
        stat.max = t;
    }
    stat.sum += i64::from(t);
    stat.count += 1;
}

#[inline(never)]
fn print(stats: BTreeMap<String, Stat>) {
    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    let mut writer = std::io::BufWriter::new(stdout);
    write!(writer, "{{").unwrap();
    let stats = BTreeMap::from_iter(
        stats
            .iter()
            // SAFETY: the README promised
            .map(|(k, v)| (unsafe { std::str::from_utf8_unchecked(k.as_ref()) }, *v)),
    );
    let mut stats = stats.into_iter().peekable();
    while let Some((station, stat)) = stats.next() {
        write!(
            writer,
            "{station}={:.1}/{:.1}/{:.1}",
            (stat.min as f64) / 10.,
            (stat.sum as f64) / 10. / (stat.count as f64),
            (stat.max as f64) / 10.
        )
        .unwrap();
        if stats.peek().is_some() {
            write!(writer, ", ").unwrap();
        }
    }
    write!(writer, "}}").unwrap();
}

#[inline]
fn parse_temperature(k: &[u8]) -> i16 {
    let tlen = k.len();
    unsafe {
        std::hint::assert_unchecked(tlen >= 3);
    }
    let isneg = std::hint::select_unpredictable(k[0] == b'-', true, false);
    let sign = i16::from(!isneg) * 2 - 1;
    let skip = usize::from(isneg);

    let isdd = std::hint::select_unpredictable(k.len() - skip == 4, true, false);
    let mul = i16::from(isdd) * 90 + 10;
    let t1 = mul * i16::from(k[skip] - b'0');
    let t2 = i16::from(isdd) * 10 * i16::from(k[tlen - 3] - b'0');
    let t3 = i16::from(k[tlen - 1] - b'0');
    sign * (t1 + t2 + t3)
}

fn split_semi(k: &[u8]) -> (&[u8], &[u8]) {
    let mut pos = k.len() - 4;
    unsafe {
        while *k.get_unchecked(pos) != b';' {
            pos -= 1;
        }
    }
    unsafe {
        let (before, after) = k.split_at_unchecked(pos + 1);
        (&before[..before.len() - 1], after)
    }
}
#[test]
fn test_split_semi_basic() {
    let input = b"hello;world";
    let (left, right) = split_semi(input);

    assert_eq!(left, b"hello");
    assert_eq!(right, b"world");
}

fn find_newline(mut map: &[u8]) -> Option<usize> {
    const LANES: usize = 32;
    const SPLAT: Simd<u8, LANES> = Simd::splat(b'\n');
    let mut i = 0;
    while let Some((chunk, rem)) = map.split_first_chunk::<32>() {
        let bytes = Simd::<u8, LANES>::from_array(*chunk);
        let mask = bytes.simd_eq(SPLAT);
        let index = mask.first_set().map(|k| k + i);
        if index.is_some() {
            return index;
        }
        i += LANES;
        map = rem;
    }
    let k = Simd::<u8, LANES>::load_or_default(map);
    k.simd_eq(SPLAT).first_set().map(|k| k + i)
}

#[test]
fn newlineworking() {
    assert_eq!(find_newline(b"HELLO\nBYE"), Some(5));
}

fn mmap(f: &File) -> &'_ [u8] {
    let len = f.metadata().unwrap().len() as usize;
    let ptr = unsafe {
        libc::mmap(
            ptr::null_mut(),
            len,
            PROT_READ,
            MAP_PRIVATE,
            f.as_raw_fd(),
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        panic!("{}", std::io::Error::last_os_error());
    } else {
        if unsafe { madvise(ptr, len, MADV_SEQUENTIAL) } != 0 {
            panic!("{}", std::io::Error::last_os_error());
        }
    }

    unsafe { std::slice::from_raw_parts(ptr as *const u8, len) }
}
