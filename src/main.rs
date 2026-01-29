#![feature(portable_simd)]
#![feature(slice_split_once)]
use std::{
    collections::{BTreeMap, HashMap},
    ffi::{c_int, c_void},
    fs::File,
    hash::{BuildHasher, Hasher},
    os::fd::AsRawFd,
    ptr, // simd::cmp::SimdPartialEq,
};

use libc::{MADV_SEQUENTIAL, MAP_SHARED, PROT_READ};

#[derive(Clone, Default)]
struct FastHashBuilder;
struct FastHasher(u64);

impl BuildHasher for FastHashBuilder {
    type Hasher = FastHasher;
    fn build_hasher(&self) -> Self::Hasher {
        FastHasher(0xcbf29ce474222325)
    }
}

impl Hasher for FastHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let (chunks, remainder) = bytes.as_chunks::<8>();
        let mut padd = [1u8; 8];
        (padd[..remainder.len()]).copy_from_slice(remainder);
        for &chunk in chunks.iter().chain(std::iter::once(&padd)) {
            let mixed = self.0 as u128 * u64::from_ne_bytes(chunk) as u128;
            self.0 = (mixed >> 64) as u64 ^ mixed as u64
        }
    }
}

// Oklahoma City;-1.0
pub fn main() {
    let f = File::open("memnts.txt").unwrap();
    let f = mmamp(&f);

    // let f = BufReader::new(f);
    //
    let mut hmap: HashMap<Vec<u8>, (i16, usize, i16, i16), FastHashBuilder> =
        HashMap::with_capacity_and_hasher(100000, FastHashBuilder);
    let mut at = 0;
    loop {
        let rest = &f[at..];
        let next_line =
            unsafe { libc::memchr(rest.as_ptr() as *const c_void, '\n' as c_int, rest.len()) };
        let k = if next_line.is_null() {
            rest
        } else {
            let len = unsafe { next_line.offset_from(rest.as_ptr() as *const c_void) as usize };
            &rest[..len]
        };
        at += k.len() + 1;
        if k.is_empty() {
            break;
        }
        // let (city, temp) = splitsc(k);
        let (city, temp) = k.rsplit_once(|d| *d == b'\t').unwrap();

        let stats = match hmap.get_mut(city) {
            Some(k) => k,
            None => hmap
                .entry(city.to_vec())
                .or_insert((i16::MAX, 0 as usize, 0 as i16, i16::MIN)),
        };
        let t = pparse(temp);
        stats.0 = stats.0.min(t);
        stats.1 += 1;
        stats.2 += i16::from(t);
        stats.3 = stats.3.max(t);
    }
    print!("{{");
    let stats = BTreeMap::from_iter(hmap);
    let mut k = stats.iter().peekable();
    while let Some((station, (min, count, sum, max))) = k.next() {
        let station = unsafe { String::from_utf8_unchecked(station.to_vec()) };
        print!(
            "{station}={:.1}/{count}/{:.1}/{:.1}",
            (*min as f64) / 10.0,
            ((*sum * *count as i16) as f64) / 10.0,
            (*max as f64) / 10.0
        );
        if k.peek().is_some() {
            print!(",");
        }
    }
    print!("}}");
}

fn mmamp(f: &File) -> &[u8] {
    let len = f.metadata().unwrap().len();
    unsafe {
        let ptr = libc::mmap(
            ptr::null_mut(),
            len as usize,
            PROT_READ,
            MAP_SHARED,
            f.as_raw_fd(),
            0,
        );

        if ptr == libc::MAP_FAILED {
            panic!("{:?}", std::io::Error::last_os_error());
        } else {
            if libc::madvise(ptr, len as libc::size_t, MADV_SEQUENTIAL) != 0 {
                panic!("{:?}", std::io::Error::last_os_error());
            }
        }

        std::slice::from_raw_parts(ptr as *const u8, len as usize)
    }
}

fn pparse(temp: &[u8]) -> i16 {
    let mut neg = false;
    let mut t = 0;
    let mut mul = 1;
    for i in temp.iter().rev() {
        match i {
            b'-' => {
                neg = true;
                break;
            }
            b'.' => {
                continue;
            }
            _ => {
                t += i16::from(i - b'0') * mul;
                mul *= 10;
            }
        }
    }
    if neg {
        t = -t;
    }
    t
}

// fn splitsc(k: &[u8]) -> (&[u8], &[u8]) {
//     if k.len() > 64 {
//         k.rsplit_once(|d| *d == b'\t').unwrap()
//     } else {
//         let delim = STALE.simd_eq(std::simd::u8x64::load_or_default(k));
//         let index = unsafe { delim.first_set().unwrap_unchecked() };
//         (&k[..index], &k[index + 1..])
//     }
// }
//
//
// #[inline(always)]
// fn splitsc(k: &[u8]) -> (&[u8], &[u8]) {
//     let i =
//         unsafe { libc::memrchr(k.as_ptr() as *const c_void, b'\t' as c_int, k.len()) } as *const u8;

//     let idx = unsafe { i.offset_from(k.as_ptr()) as usize };
//     (&k[..idx], &k[idx + 1..])
// }
