/// Fixed number definition
pub type Fixed = fixed::types::I24F40;

/// Expected Fixed number bytes
pub type NumberBytes = [u8; 8];

pub fn to_fixed(bytes:NumberBytes) -> Fixed {
    Fixed::from_be_bytes(bytes)
}

pub fn to_bytes(fixed:Fixed) -> NumberBytes {
    Fixed::to_be_bytes(fixed)
}

pub fn tick_volatility(ticks: &[Fixed],n_inv_sqrt:Fixed,n1_inv:Fixed) -> Fixed {
    let mut ticks_prev = ticks[0];
    let (sum_u, sum_u2) =
        ticks
            .iter()
            .skip(1)
            .fold((Fixed::ZERO, Fixed::ZERO), |(su, su2), ticks_curr| {
                let delta = ticks_curr - ticks_prev;
                ticks_prev = *ticks_curr;
                (su + delta * n_inv_sqrt, su2 + delta * delta * n1_inv)
            });
    
    sum_u2 - (sum_u * sum_u) * n1_inv
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_bytes_conversion() {
        let bytes = [255, 255, 255, 255, 255, 204, 133, 221];
        let fixed = to_fixed(bytes);
        let bytes2 = to_bytes(fixed);
        let fixed2 = to_fixed(bytes2);
        assert_eq!(bytes, bytes2);
        assert_eq!(fixed, fixed2);
    }

}