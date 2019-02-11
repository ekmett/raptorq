use octet::Octet;
use octet::OCTET_MUL;
use octet::OCTET_MUL_LOW_BITS;
use octet::OCTET_MUL_HI_BITS;

#[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2")))]
fn mulassign_scalar_fallback(octets: &mut Vec<u8>, scalar: &Octet) {
    let scalar_index = (scalar.byte() as usize) << 8;
    for i in 0..octets.len() {
        unsafe {
            *octets.get_unchecked_mut(i) = *OCTET_MUL.get_unchecked(scalar_index + *octets.get_unchecked(i) as usize);
        }
    }
}

#[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2"))]
fn mulassign_scalar_avx2(octets: &mut Vec<u8>, scalar: &Octet) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let low_mask;
    let hi_mask;
    unsafe {
        low_mask =_mm256_set1_epi8(0x0F);
        hi_mask = _mm256_set1_epi8(0xF0 as u8 as i8);
    }
    let self_avx_ptr = octets.as_mut_ptr() as *mut __m256i;
    let low_table;
    let hi_table;
    unsafe  {
        low_table =_mm256_loadu_si256(OCTET_MUL_LOW_BITS[scalar.byte() as usize].as_ptr() as *const __m256i);
        hi_table =_mm256_loadu_si256(OCTET_MUL_HI_BITS[scalar.byte() as usize].as_ptr() as *const __m256i);
    }

    for i in 0..(octets.len() / 32) {
        unsafe {
            let self_vec = _mm256_loadu_si256(self_avx_ptr.add(i));
            let low = _mm256_and_si256(self_vec, low_mask);
            let low_result = _mm256_shuffle_epi8(low_table, low);
            let hi = _mm256_and_si256(self_vec, hi_mask);
            let hi = _mm256_srli_epi64(hi, 4);
            let hi_result = _mm256_shuffle_epi8(hi_table, hi);
            let result = _mm256_xor_si256(hi_result, low_result);
            _mm256_storeu_si256(self_avx_ptr.add(i), result);
        }
    }

    let remainder = octets.len() % 32;
    let scalar_index = (scalar.byte() as usize) << 8;
    for i in (octets.len() - remainder)..octets.len() {
        unsafe {
            *octets.get_unchecked_mut(i) = *OCTET_MUL.get_unchecked(scalar_index + *octets.get_unchecked(i) as usize);
        }
    }
}

pub fn mulassign_scalar(octets: &mut Vec<u8>, scalar: &Octet) {
    unsafe {
        assert_ne!(0, OCTET_MUL[1 << 8 | 1], "Must call Octet::static_init()");
    }
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2"))]
    return mulassign_scalar_avx2(octets, scalar);

    #[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2")))]
    return mulassign_scalar_fallback(octets, scalar);
}

#[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2")))]
fn fused_addassign_mul_scalar_fallback(octets: &mut Vec<u8>, other: &Vec<u8>, scalar: &Octet) {
    let scalar_index = (scalar.byte() as usize) << 8;
    for i in 0..octets.len() {
        unsafe  {
            *octets.get_unchecked_mut(i) ^= *OCTET_MUL.get_unchecked(scalar_index + *other.get_unchecked(i) as usize);
        }
    }
}

#[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2"))]
fn fused_addassign_mul_scalar_avx2(octets: &mut Vec<u8>, other: &Vec<u8>, scalar: &Octet) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let low_mask;
    let hi_mask;
    unsafe {
        low_mask =_mm256_set1_epi8(0x0F);
        hi_mask = _mm256_set1_epi8(0xF0 as u8 as i8);
    }
    let self_avx_ptr = octets.as_mut_ptr() as *mut __m256i;
    let other_avx_ptr = other.as_ptr() as *const __m256i;
    let low_table;
    let hi_table;
    unsafe  {
        low_table =_mm256_loadu_si256(OCTET_MUL_LOW_BITS[scalar.byte() as usize].as_ptr() as *const __m256i);
        hi_table =_mm256_loadu_si256(OCTET_MUL_HI_BITS[scalar.byte() as usize].as_ptr() as *const __m256i);
    }

    for i in 0..(octets.len() / 32) {
        unsafe {
            // Multiply by scalar
            let other_vec = _mm256_loadu_si256(other_avx_ptr.add(i));
            let low = _mm256_and_si256(other_vec, low_mask);
            let low_result = _mm256_shuffle_epi8(low_table, low);
            let hi = _mm256_and_si256(other_vec, hi_mask);
            let hi = _mm256_srli_epi64(hi, 4);
            let hi_result = _mm256_shuffle_epi8(hi_table, hi);
            let other_vec = _mm256_xor_si256(hi_result, low_result);

            // Add to self
            let self_vec = _mm256_loadu_si256(self_avx_ptr.add(i));
            let result = _mm256_xor_si256(self_vec, other_vec);
            _mm256_storeu_si256(self_avx_ptr.add(i), result);
        }
    }

    let remainder = octets.len() % 32;
    let scalar_index = (scalar.byte() as usize) << 8;
    for i in (octets.len() - remainder)..octets.len() {
        unsafe  {
            *octets.get_unchecked_mut(i) ^= *OCTET_MUL.get_unchecked(scalar_index + *other.get_unchecked(i) as usize);
        }
    }
}

pub fn fused_addassign_mul_scalar(octets: &mut Vec<u8>, other: &Vec<u8>, scalar: &Octet) {
    // TODO: enable these in debug only?
    assert_ne!(*scalar, Octet::one(), "Don't call this with one. Use += instead");
    assert_ne!(*scalar, Octet::zero(), "Don't call with zero. It's very inefficient");

    unsafe {
        assert_ne!(0, OCTET_MUL[1 << 8 | 1], "Must call Octet::static_init()");
    }

    assert_eq!(octets.len(), other.len());
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2"))]
    return fused_addassign_mul_scalar_avx2(octets, other, scalar);

    #[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2")))]
    return fused_addassign_mul_scalar_fallback(octets, other, scalar);
}

#[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2")))]
fn add_assign_fallback(octets: &mut Vec<u8>, other: &Vec<u8>) {
    assert_eq!(octets.len(), other.len());
    let self_ptr = octets.as_mut_ptr() as *mut u64;
    let other_ptr = other.as_ptr() as *const u64;
    for i in 0..(octets.len() / 8) {
        unsafe {
            *self_ptr.add(i) ^= *other_ptr.add(i);
        }
    }
    let remainder = octets.len() % 8;
    for i in (octets.len() - remainder)..octets.len() {
        unsafe {
            *octets.get_unchecked_mut(i) ^= other.get_unchecked(i);
        }
    }
}

#[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2"))]
fn add_assign_avx2(octets: &mut Vec<u8>, other: &Vec<u8>) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    assert_eq!(octets.len(), other.len());
    let self_avx_ptr = octets.as_mut_ptr() as *mut __m256i;
    let other_avx_ptr = other.as_ptr() as *const __m256i;
    for i in 0..(octets.len() / 32) {
        unsafe {
            let self_vec = _mm256_loadu_si256(self_avx_ptr.add(i));
            let other_vec = _mm256_loadu_si256(other_avx_ptr.add(i));
            let result = _mm256_xor_si256(self_vec, other_vec);
            _mm256_storeu_si256(self_avx_ptr.add(i), result);
        }
    }

    let remainder = octets.len() % 32;
    let self_ptr = octets.as_mut_ptr() as *mut u64;
    let other_ptr = other.as_ptr() as *const u64;
    for i in ((octets.len() - remainder) / 8)..(octets.len() / 8) {
        unsafe {
            *self_ptr.add(i) ^= *other_ptr.add(i);
        }
    }

    let remainder = octets.len() % 8;
    for i in (octets.len() - remainder)..octets.len() {
        unsafe {
            *octets.get_unchecked_mut(i) ^= other.get_unchecked(i);
        }
    }
}

pub fn add_assign(octets: &mut Vec<u8>, other: &Vec<u8>) {
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2"))]
    return add_assign_avx2(octets, other);

    #[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2")))]
    return add_assign_fallback(octets, other);
}

#[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2"))]
#[inline(never)]
fn count_ones_and_nonzeros_avx2(octets: &[u8]) -> (u32, u32) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let avx_ones;
    let avx_zeros;
    unsafe {
        avx_ones =_mm256_set1_epi8(1);
        avx_zeros = _mm256_set1_epi8(0);
    }
    let avx_ptr = octets.as_ptr() as *const __m256i;

    let mut ones = 0;
    let mut non_zeros = 0;
    for i in 0..(octets.len() / 32) {
        unsafe {
            let vec = _mm256_loadu_si256(avx_ptr.add(i));
            let compared_ones = _mm256_cmpeq_epi8(vec, avx_ones);
            ones += _mm256_extract_epi64(compared_ones, 0).count_ones() / 8;
            ones += _mm256_extract_epi64(compared_ones, 1).count_ones() / 8;
            ones += _mm256_extract_epi64(compared_ones, 2).count_ones() / 8;
            ones += _mm256_extract_epi64(compared_ones, 3).count_ones() / 8;

            let compared_zeros = _mm256_cmpeq_epi8(vec, avx_zeros);
            non_zeros += 32;
            non_zeros -= _mm256_extract_epi64(compared_zeros, 0).count_ones() / 8;
            non_zeros -= _mm256_extract_epi64(compared_zeros, 1).count_ones() / 8;
            non_zeros -= _mm256_extract_epi64(compared_zeros, 2).count_ones() / 8;
            non_zeros -= _mm256_extract_epi64(compared_zeros, 3).count_ones() / 8;
        }
    }

    let mut remainder = octets.len() % 32;
    if remainder >= 16 {
        remainder -= 16;
        let avx_ones;
        let avx_zeros;
        unsafe {
            avx_ones =_mm_set1_epi8(1);
            avx_zeros = _mm_set1_epi8(0);
        }
        let avx_ptr;
        unsafe {
            avx_ptr = octets.as_ptr().add((octets.len() / 32) * 32) as *const __m128i;
        }

        unsafe {
            let vec = _mm_lddqu_si128(avx_ptr);
            let compared_ones = _mm_cmpeq_epi8(vec, avx_ones);
            ones += _mm_extract_epi64(compared_ones, 0).count_ones() / 8;
            ones += _mm_extract_epi64(compared_ones, 1).count_ones() / 8;

            let compared_zeros = _mm_cmpeq_epi8(vec, avx_zeros);
            non_zeros += 16;
            non_zeros -= _mm_extract_epi64(compared_zeros, 0).count_ones() / 8;
            non_zeros -= _mm_extract_epi64(compared_zeros, 1).count_ones() / 8;
        }
    }

    for i in (octets.len() - remainder)..octets.len() {
        let value;
        unsafe {
            value = octets.get_unchecked(i);
        }
        if *value == 1 {
            ones += 1;
        }
        if *value != 0 {
            non_zeros += 1;
        }
    }
    (ones, non_zeros)
}

#[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2")))]
fn count_ones_and_nonzeros_fallback(octets: &[u8]) -> (u32, u32) {
    let mut ones = 0;
    let mut non_zeros = 0;
    for value in octets.iter() {
        if *value == 1 {
            ones += 1;
        }
        if *value != 0 {
            non_zeros += 1;
        }
    }
    (ones, non_zeros)
}

pub fn count_ones_and_nonzeros(octets: &[u8]) -> (u32, u32) {
    #[cfg(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2"))]
    return count_ones_and_nonzeros_avx2(octets);

    #[cfg(not(all(any(target_arch = "x86", target_arch = "x86_64"), target_feature = "avx2")))]
    return count_ones_and_nonzeros_fallback(octets);
}

#[cfg(test)]
mod tests {
    extern crate rand;

    use octets::tests::rand::Rng;
    use octet::Octet;
    use octets::fused_addassign_mul_scalar;
    use octets::mulassign_scalar;

    #[test]
    fn mul_assign() {
        let size = 41;
        let scalar = Octet::new(rand::thread_rng().gen_range(1, 255));
        let mut data1: Vec<u8> = vec![0; size];
        let mut expected: Vec<u8> = vec![0; size];
        for i in 0..size {
            data1[i] = rand::thread_rng().gen();
            expected[i] = (&Octet::new(data1[i]) * &scalar).byte();
        }

        mulassign_scalar(&mut data1, &scalar);

        assert_eq!(expected, data1);
    }

    #[test]
    fn fma() {
        let size = 41;
        let scalar = Octet::new(rand::thread_rng().gen_range(1, 255));
        let mut data1: Vec<u8> = vec![0; size];
        let mut data2: Vec<u8> = vec![0; size];
        let mut expected: Vec<u8> = vec![0; size];
        for i in 0..size {
            data1[i] = rand::thread_rng().gen();
            data2[i] = rand::thread_rng().gen();
            expected[i] = (Octet::new(data1[i]) + &Octet::new(data2[i]) * &scalar).byte();
        }

        fused_addassign_mul_scalar(&mut data1, &data2, &scalar);

        assert_eq!(expected, data1);
    }
}
