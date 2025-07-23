use rand::{rng, Rng};

#[inline(always)]
pub fn gen_seed(num: usize) -> Vec<u8> {
    let mut rng = rng();
    let mut res = Vec::new();
    for _ in 0..num {
        res.push(rng.random::<u8>());
    }

    res
}
