use core::f64;
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{BufRead, BufReader},
};

// Oklahoma City;-1.0
pub fn main() {
    let f = File::open("memnts.txt").unwrap();
    let f = BufReader::new(f);
    let mut hmap: HashMap<Vec<u8>, (f64, usize, f64, f64)> = HashMap::new();
    for k in f.split(b'\n') {
        let k = k.unwrap();
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
