
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Volatility {
    pub n: usize,
    pub n_inv_sqrt: f32,
    pub n1_inv: f32,
    pub s2: f32,
}

const THREE_HALFS: f32 = 1.5;
const WTF: u32 = 0x5f3759df;


// See https://en.wikipedia.org/wiki/Fast_inverse_square_root
// Originally implemented by game developer legend John Carmack in Quake III Arena
// TT: Left the original comments for respect to the original author
fn q_inv_sqrt(value:f32) -> f32 {
    let mut y = value;
    let mut i: u32;
    let x2: f32 = value * 0.5;

    // Evil bit hack
    i = y.to_bits();

    // What the f*ck
    i = WTF - (i >> 1);

    y = f32::from_bits(i);

    // Newton iteration
    y = y * (THREE_HALFS - (x2 * y * y));

    y = y * (THREE_HALFS - (x2 * y * y)); // 2nd iteration, this can be removed
    
    y
}


impl Volatility {

     pub fn new(ticks: &[f32]) -> Self {
        let n = ticks.len();
        let n_inv_sqrt = q_inv_sqrt(n as f32);
        let n1_inv = 1.0f32 / n as f32;
        let mut ticks_prev = ticks[0];
        
        let mut sum_u = 0f32;
        let mut sum_u2 = 0f32;

        for i in 1..n {
            let delta = ticks[i] - ticks_prev;
            ticks_prev = ticks[i];
            sum_u += delta * n_inv_sqrt;
            sum_u2 += delta * delta * n1_inv;
        }
        
        let s2 = sum_u2 - (sum_u * sum_u) * n1_inv;
    
        Self {
            n,
            n_inv_sqrt,
            n1_inv,
            s2,
        }
    }
}
