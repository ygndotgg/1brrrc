use core::f64;
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufRead, BufReader},
    os::fd::AsRawFd,
    ptr,
};

use libc::{MADV_SEQUENTIAL, MAP_SHARED, PROT_READ, PTHREAD_RECURSIVE_MUTEX_INITIALIZER_NP};

// Oklahoma City;-1.0
pub fn main() {
    let f = File::open("memnts.txt").unwrap();
    let f = mmamp(&f);
    // let f = BufReader::new(f);
    let mut hmap: HashMap<Vec<u8>, (f64, usize, f64, f64)> = HashMap::new();
    for k in f.split(|a| *a == b'\n') {
        if k.is_empty() {
            break;
        }
        let mut k = k.rsplitn(2, |k| *k == b';');

        let temp = k.next().unwrap();
        let city = k.next().unwrap();
        let stats = match hmap.get_mut(city) {
            Some(k) => k,
            None => hmap
                .entry(city.to_vec())
                .or_insert((f64::MAX, 0 as usize, 0 as f64, f64::MIN)),
        };
        let temp: f64 = unsafe { std::str::from_utf8_unchecked(temp) }
            .parse()
            .unwrap();
        stats.0 = stats.0.min(temp);
        stats.1 += 1;
        stats.2 += temp;
        stats.3 = stats.3.max(temp);
    }
    print!("{{");
    let stats = BTreeMap::from_iter(hmap);
    let mut k = stats.iter().peekable();
    while let Some((station, (min, count, sum, max))) = k.next() {
        let station = unsafe { String::from_utf8_unchecked(station.to_vec()) };
        print!("{station}={min:.1}/{count}/{sum:.1}/{max:.1}");
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
