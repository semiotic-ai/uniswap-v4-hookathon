use halo2_base::utils::ScalarField;

pub trait ScalarFieldExt {

    /// Gets the least significant 128 bits of the field element.
    fn get_lower_128(&self) -> u128;

}

impl<F:ScalarField> ScalarFieldExt for F {

    fn get_lower_128(&self) -> u128
    {
        let bytes = self.to_bytes_le();
        let mut lower_128 = 0u128;
        for (i, byte) in bytes.into_iter().enumerate().take(16) {
            lower_128 |= (byte as u128) << (i * 8);
        }
        lower_128
    }
}


#[derive(Default)]
pub struct State {
    pub n:f64,
    pub delta_sq_sum:f64,
    pub first:Option<f64>,
    pub prev:Option<f64>,
}

impl State {

    pub fn volatility(&self) -> f64 {

        let delta = self.prev.unwrap_or_default() - self.first.unwrap_or_default();
        (self.delta_sq_sum - ((delta * delta)/ self.n)) / (self.n - 1f64)
    }

    pub fn update(mut self, tick:f64) -> Self {
       self.n += 1f64; 
       if let Some(prev) = self.prev {
           let delta = tick - prev;
           self.delta_sq_sum += delta * delta;
       } 
       else if self.first.is_none() {
           self.first = Some(tick);
       }
       self.prev = Some(tick);
       self
    }
}

pub fn calculate_optimized(ticks: &[f64]) -> f64 {
    
    let state = ticks.into_iter()
        .fold(State::default(), |s,t | s.update(*t));
  
    state.volatility()

}


pub fn calculate_original(ticks: &[f64]) -> f64 {
    let n = ticks.len() as f64;
    let n_inv_sqrt = 1f64 / n.sqrt();
    let n1_inv = 1f64 / (n - 1f64);
    let mut ticks_prev = ticks[0];
    let (sum_u, sum_u2) =
        ticks
            .iter()
            .skip(1)
            .fold((0f64, 0f64), |(su, su2), ticks_curr| {
                let delta = ticks_curr - ticks_prev;
                ticks_prev = *ticks_curr;
                (su + delta * n_inv_sqrt, su2 + delta * delta * n1_inv)
            });
    sum_u2 - (sum_u * sum_u) * n1_inv    
}

