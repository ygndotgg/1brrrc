use std::{
    collections::{BTreeMap, HashMap},
    ffi::{c_int, c_void},
    fs::File,
    os::fd::AsRawFd,
    ptr,
};

use libc::{MADV_SEQUENTIAL, MAP_SHARED, PROT_READ};

// Oklahoma City;-1.0
pub fn main() {
    let f = File::open("memnts.txt").unwrap();
    let f = mmamp(&f);

    // let f = BufReader::new(f);
    let mut hmap: HashMap<Vec<u8>, (i16, usize, i16, i16)> = HashMap::new();
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
        let mut k = k.rsplitn(2, |k| *k == b';');

        let temp = k.next().unwrap();
        let city = k.next().unwrap();
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
