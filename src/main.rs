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
    let mut hmap: HashMap<String, (f64, usize, f64, f64)> = HashMap::new();
    for k in f.lines() {
        let k = k.unwrap();
        let (city, temp) = k.split_once(";").unwrap();
        let stats = match hmap.get_mut(city) {
            Some(k) => k,
            None => {
                hmap.entry(city.to_string())
                    .or_insert((f64::MAX, 0 as usize, 0 as f64, f64::MIN))
            }
        };
        let temp: f64 = temp.parse().unwrap();
        stats.0 = stats.0.min(temp);
        stats.1 += 1;
        stats.2 += stats.2;
        stats.3 = stats.3.max(temp);
    }
    print!("{{");
    let stats = BTreeMap::from_iter(hmap);
    let mut k = stats.iter().peekable();
    while let Some((station, (min, count, sum, max))) = k.next() {
        print!("{station}={min:.1}/{count}/{sum:.1}/{max:.1}");
        if k.peek().is_some() {
            print!(",");
        }
    }
    print!("}}");
}
