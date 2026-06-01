use rand::{rng, RngExt};

#[inline(always)]
pub fn gen_seed(num: usize) -> Vec<u8> {
    let mut res = vec![0u8; num];
    rng().fill(&mut res);
    res
}
